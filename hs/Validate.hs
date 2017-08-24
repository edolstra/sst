module Validate (validate, Schema(..), Proof(..)) where

import AST
import Data.Map
import Data.Maybe

data Schema
  = SChoice [Schema]
  | SSeq [Schema]
  | SMany Schema
  | STag String (Map String Schema) [Schema]
  | SText
  | SPara Schema
  | SInt
  deriving Show

data Proof
  = PChoice Int Proof
  | PSeq [Proof]
  | PMany [Proof]
  | PTag String (Map String Proof) [Proof]
  | PText String
  | PPara Proof
  | PInt Int
  | PAny Schema [Item]
  deriving Show

validate :: Schema -> Doc -> Maybe Proof
validate schema (Doc doc)
  = case val schema doc of
      Just (p, []) -> Just p
      -- Ignore whitespace at the end of the document.
      Just (p, [Text s]) | isWS s -> Just p
      _ -> Nothing

  where

    val :: Schema -> [Item] -> Maybe (Proof, [Item])

    val (SChoice ss) xs
      = val' 0 ss xs
      where
        val' _ [] xs = Nothing
        val' n (s:ss) xs = maybe
          (val' (n+1) ss xs)
          (\(p, xs') -> Just (PChoice n p, xs'))
          (val s xs)

    val (SSeq []) xs
      = Just (PSeq [], xs)

    val (SSeq (s:ss)) xs
      = do (p, xs') <- val s xs
           (PSeq ps, xs'') <- val (SSeq ss) xs'
           return $ (PSeq $ p : ps, xs'')

    val (SMany s) xs
      | isJust res
      = Just (PMany $ p : ps, xs'')
      where
        res = val s xs
        Just (p, xs') = res
        Just (PMany ps, xs'') = val (SMany s) xs'

    val (SMany s) xs
      = Just (PMany [], xs)

    val s@(STag _ _ _) (Text t : xs)
      | isWS t
      = val s xs

    val (STag name expArgs args) (Tag name' expArgs' args' : xs)
      | name == name'
        && length args == length args'
        && all isJust ps
        -- FIXME: handle expArgs
      = Just (PTag name empty (Prelude.map fromJust ps), xs)
      where ps = zipWith (\s a -> validate s a) args args'

    val SText (Text s : xs)
      = Just (PText s, xs)

    val (SPara s) xs
      | length as > 0 && isJust p
      = Just (PPara $ fromJust p, bs)
      where

        (as, bs) = eatPara xs'

        p = validate s (Doc as)
        --p = Just $ PAny s as

        -- Skip leading whitespace.
        xs' = case xs of
          Text t : ys -> putBack (skipWS t) ys
          ys -> ys

        eatPara (Text ('\n':'\n':cs) : xs)
          = ([], putBack cs xs)
        eatPara (Text (c:cs) : xs)
          = (putBack [c] as, bs)
          where (as, bs) = eatPara (Text cs : xs)
        eatPara (Text [] : xs)
          = eatPara xs
        eatPara (x:xs)
          = (x : as, bs)
          where (as, bs) = eatPara xs
        eatPara [] = ([], [])

    val SInt (Text s : xs) | length n > 0
      = Just (PInt $ read n, putBack s' xs)
      where
        -- FIXME: integer should be followed by whitespace or EOF or tag
        (n, s') = span (\s -> elem s ['0'..'9']) (skipWS s)

    val _ xs = Nothing

isText (Text _) = True
isText _ = False

isWS = Prelude.null . skipWS

skipWS = dropWhile (\s -> elem s " \n\r\t")

putBack s xs =
  if Prelude.null s
  then xs
  else case xs of
    Text t : xs -> Text (s ++ t) : xs
    xs -> Text s : xs
