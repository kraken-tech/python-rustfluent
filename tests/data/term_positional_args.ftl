-brand-name = { $case ->
   *[nominative] Firefox
    [locative] Firefoxie
}

# This message uses a positional argument to a term (which is ignored per spec)
bad-reference = Visit { -brand-name("positional") } for more info.

# This message uses both positional and named arguments
bad-reference-mixed = About { -brand-name("ignored", case: "locative") }.

# This is the correct way - only named arguments
good-reference = About { -brand-name(case: "locative") }.
