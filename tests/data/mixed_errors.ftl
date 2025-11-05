# File with both parse errors and validation errors

# This is a parse error - missing =
invalid-syntax

# This is valid but references unknown message
valid-with-bad-ref = Check { unknown-msg }

# This is valid
good-message = This is fine
