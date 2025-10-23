# Test file for Fluent attributes
welcome-message = Welcome!
    .title = Welcome to our site
    .aria-label = Welcome greeting

login-input = Email
    .placeholder = email@example.com
    .aria-label = Login input value
    .title = Type your login email

# Message with variables in attributes
greeting = Hello
    .formal = Hello, { $name }
    .informal = Hi { $name }!

# Message with only attributes (no value)
form-button =
    .submit = Submit Form
    .cancel = Cancel
    .reset = Reset Form
