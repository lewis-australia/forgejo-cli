test-hello = Hello
test-fallback-only-english = This message is only in english

test-placeables = Hello, {$name}! You're {$name}.

test-functions = This text is {STYLE("red")}red!{STYLE("none")}

test-switch = { $n -> 
    [one] A thing
   *[other] { $n } things
}

test-switch-2 = { 30.0 -> 
    [one] A thing
   *[other] { $n } things
}
