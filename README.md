# Cursed Lox

It's like Lox but...

- all the identifers start with a dollar sign

# Test suite

Tests are copied from https://github.com/munificent/craftinginterpreters/tree/master/test.

# Instruments

Run `codesign -s - -v -f --entitlements debug.plist target/release/lox` to codesign the release binary.
This will allow instruments to work properly.

