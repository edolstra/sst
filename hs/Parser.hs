module Parser (parseFile) where

import AST
import Data.Map
import Text.ParserCombinators.Parsec
import Text.Printf

parseFile :: String -> IO (Either ParseError Doc)
parseFile filename = parseFromFile doc filename

doc :: GenParser Char st Doc
doc =
  do xs <- many item
     eof
     return $ Doc (flatten xs)

item :: GenParser Char st Item
-- FIXME: flattening {{...}} into a Text node causes it to be searched
-- for paragraph breaks. Desirable?
item = text <|> (raw >>= \s -> return $ Text s) <|> beginEnd <|> tag

text :: GenParser Char st Item
text =
  do cs <- many1 (noneOf "\\{}[]" <|> try escape)
     return $ Text cs

escape :: GenParser Char st Char
escape =
  do char '\\'
     c <- oneOf "\\{}[]"
     return c

tag :: GenParser Char st Item
tag =
  do name <- try (char '\\' >> elementName)
     namedArgs <- many namedArg
     args <- many arg
     return $ Tag name (fromList namedArgs) args

beginEnd :: GenParser Char st Item
beginEnd =
  do name <- tag' "begin"

     namedArgs <- many namedArg
     args <- many arg
     xs <- many item

     name' <- tag' "end"

     if name /= name' then
       fail $ printf "begin tag '%s' does not match end tag '%s'" name name'
     else
       return $ Tag name (fromList namedArgs) (args ++ [Doc (flatten xs)])

  where
    tag' what =
      do try $ char '\\' >> string what
         ws
         char '{'
         ws
         name <- elementName
         ws
         char '}'
         return name

arg :: GenParser Char st Doc
arg =
  do try $ ws >> char '{'
     xs <- many item
     char '}'
     return $ Doc (flatten xs)

raw :: GenParser Char st String
raw =
  do try $ string "{{"
     xs <- many (
       (noneOf "{}" >>= \c -> return [c])
       <|> try (char '}' >> noneOf "}" >>= \c -> return ['}', c])
       <|> try (char '{' >> noneOf "{" >>= \c -> return ['{', c])
       <|> (raw >>= \s -> return $ "{{" ++ s ++ "}}")
       )
     string "}}"
     return $ concat xs

namedArg :: GenParser Char st (String, Doc)
namedArg =
  do try $ ws >> char '['
     ws
     name <- elementName
     ws
     char '='
     xs <- many item
     char ']'
     return (name, Doc (flatten xs))

elementName :: GenParser Char st String
elementName =
  do name <- many1 (oneOf (['a'..'z'] ++ ['A'..'Z'] ++ ['0'..'9'] ++ ['#']))
     if name == "begin" || name == "end" then
       fail $ printf "invalid element name '%s'" name
     else
       return name

ws = many (oneOf " \t\n\r")
