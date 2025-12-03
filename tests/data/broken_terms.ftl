# File with broken term references for testing validation

-valid-term = This is a valid term
    .case = nominative

# Message references non-existent term
broken-reference = This references { -nonexistent-term }

# Message uses non-existent term attribute as selector
broken-attribute = { -valid-term.nonexistent ->
    [value] Something
   *[other] Other
}
