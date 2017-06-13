{-# LANGUAGE OverloadedStrings    #-}

module Main where

import Control.Exception (throwIO)
import System.Environment

import Network.HTTP.Req

instance MonadHttp IO where
  handleHttpException = throwIO

main :: IO ()
main = do
  args <- getArgs
  let query = head args
  res <- req GET
    (https "emojipedia.org" /: "search")
    NoReqBody
    bsResponse
    ("q" =: query)
  print (responseBody res)
