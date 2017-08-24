module AST (Doc(..), Item(..), flatten) where

import Data.Map

data Doc = Doc [Item]
  deriving (Show, Eq)

data Item
  = Text String
  | Tag String (Map String Doc) [Doc]
  deriving (Show, Eq)

-- Helper function to squash adjacent Text nodes.
flatten :: [Item] -> [Item]
flatten (Text s1 : Text s2 : xs) = flatten (Text (s1 ++ s2) : xs)
flatten (x:xs) = x : flatten xs
flatten [] = []
