# ranim

simple plugin that implements saving and loading of anims 

## RSaveRecordedAnimation
`void functionref(var recording, string name)`

just kind of saves the file to disk under this name

### Errors
will throw an error if it isn't able to save the file

## RReadRecordedAnimation
`var functionref(string name)`

just kind of loads the file from disk under this name

### Errors
will throw an error if it isn't able to read the file


#### TODO
add recursive search to find recordings shipped with mods
