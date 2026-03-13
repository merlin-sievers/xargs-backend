import htmlgen
import jester

import os, strutils
import asyncdispatch

type Game = object
    name : string

router rt:
    get "/highscore/@game":
        resp "Hi!"

proc main() =
    let port = paramStr(1).parseInt().Port
    let settings = newSettings(port=port)
    var jester = initJester(rt, settings=settings)
    jester.serve()

dispatch main
