module Main where

import System.Environment

main :: IO ()
main = do
  args <- getArgs
  let query = head args
  putStrLn query
