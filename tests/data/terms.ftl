# Test file with Fluent terms
-brand-name = Acme Corporation

-product-name = Super Widget
    .category = gadget

# Messages that reference terms
welcome = Welcome to { -brand-name }!
product-info = Learn about { -product-name }

# Message using term attribute as selector (this is valid per Fluent spec)
product-category = { -product-name.category ->
    [gadget] This is a gadget
   *[other] This is something else
}
