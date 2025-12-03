# This has a cyclic reference
msg-a = Value: { msg-b }
msg-b = Value: { msg-a }
