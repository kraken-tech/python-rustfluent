# Messages for testing variable extraction and validation

# Simple message with one variable
greeting = Hello, { $name }!

# Message with multiple variables
user-info = { $username } has { $count } messages

# Message with no variables
static-message = This has no variables

# Message with nested variables in selector
item-status = { $count ->
    [0] No items for { $user }
    [1] One item for { $user }
   *[other] { $count } items for { $user }
}

# Message attribute with variables
email-template = Send email
    .subject = Hello { $recipient }
    .body = You have { $messageCount } new messages
