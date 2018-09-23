module Eval (eval, Context(..)) where

import AST
import Data.Map
import Text.Printf

type Env = Map String Macro

data Context = Context
  { getDoc :: String -> String -> Doc
  , getFile :: String -> String -> String
  }

data Macro = Macro Int (Map String Doc) Doc Env

eval :: Context -> String -> Doc -> Doc
eval ctx filename doc = eval' empty doc

  where

    eval' :: Env -> Doc -> Doc
    eval' env (Doc xs) = Doc $ flatten $ eval'' env xs

    eval'' :: Env -> [Item] -> [Item]

    eval'' env (t@(Text _) : xs)
      = t : eval'' env xs

    eval'' env (Tag "def" namedArgs ((Doc [Text name]) : body : []) : xs)
      = eval'' (insert name (Macro arity expNamedArgs body env) env) xs
      where
        arityS = findWithDefault (Doc [Text "0"]) "arity" namedArgs
        arity = case arityS of
          Doc [Text s] -> read s :: Int
        expNamedArgs = delete "arity" namedArgs

    eval'' env (Tag "#" _ _ : xs)
      = eval'' env xs

    eval'' env (Tag "strip" _ [Doc d] : xs)
      = d ++ eval'' env xs

    eval'' env (Tag "include" _ ((Doc [Text filename']) : []) : xs)
      = eval'' empty (extract (getDoc ctx filename filename')) ++ eval'' env xs
      where extract (Doc doc) = doc

    eval'' env (Tag "includeraw" _ ((Doc [Text filename']) : []) : xs)
      = Text (getFile ctx filename filename') : eval'' env xs

    eval'' env (t@(Tag name namedArgs args) : xs)
      = case Data.Map.lookup name env of
          Nothing ->
            Tag name
              (Data.Map.map (eval' env) namedArgs)
              (Prelude.map (eval' env) args)
            : eval'' env xs
          Just (Macro arity expNamedArgs (Doc body) env') ->
            -- FIXME: check for undeclared args
            if arity /= length args && not (arity == 0 && args == [Doc []]) then
              error $ printf "macro '%s' called with %v arguments but it expects %v"
                name (show $ length args) (show arity)
            else
              (eval'' env'' body) ++ eval'' env xs
            where env'' =
                    union
                      (fromList $ zipWith (\n v -> (show n, toMacro v)) [0..] args)
                      (union (Data.Map.mapWithKey (\n v -> toMacro (findWithDefault v n namedArgs)) expNamedArgs) env')

    eval'' env [] = []

toMacro v = Macro 0 empty v empty // FIXME: should be evaluated in macro's env?
