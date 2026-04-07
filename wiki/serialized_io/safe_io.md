# safe io
a better implementation of safe io with serialization and no callbacks


## `BPLoadFileAsync<TypeName>`
`void functionref(string file_name, array<TypeName> out_param, array<string> ornull out_error)`

Given a file it will read out and deserialize the contents for a given type.
The output will override the 0th index of the out_param array. This function will
override the contents of the file.

**This function must be called from a thread**, since it will pause the current squirrel thread to
asynchronously load the file.

```c
array<SerializableStruct> out;

BPLoadFileAsyncSerializableStruct("test", out, null)

print( out[0].a )
```

### Errors
errors are optionally reported in the out_error out parameter.

## `BPSaveFileAsync<TypeName>`
`void functionref(string file_name, TypeName contents, array<string> ornull out_error)`

Given a file it will save the contents provided and serialize them for a given type.

**This function must be called from a thread**, since it will pause the current squirrel thread to
asynchronously save the file.

```c
BPSaveFileAsyncStringArray("array", [ "1", "2", "3", "4" ], null)
```

### Errors
errors are optionally reported in the out_error out parameter.

## `BPLoadFile<TypeName>`
`string ornull functionref(string file_name, array<TypeName> out_param)`

Given a file it load the file contents into the out_param. This function will override the contents of the file.

This is a sync version of `BPLoadFileAsync` as such it can block the whole game. 

### Errors
errors are returned as string, null indicates no errors occurred 

## `BPSaveFile<TypeName>`
`string ornull functionref(string file_name, TypeName contents)`

Given a file it will save the provided contents.

This is a sync version of `BPSaveFileAsync` as such it can block the whole game. 

### Errors
errors are returned as string, null indicates no errors occurred 

## `BPDeleteFile` 
`string ornull functionref(string file_name)`

Will delete the provided file from disc

### Errors
errors are returned as string, null indicates no errors occurred 

## `BPDeleteFile` 
`string ornull functionref(string file_name)`

Will delete the provided file from disc

## `BPDoesFileExist` 
`string ornull functionref(string file_name)`

Will check if the provided file name exists in the file system.

### Errors
errors are returned as string, null indicates no errors occurred 


## Note
These functions can be used without `<TypeName>` suffix which will by default return a string of type `var` for loading and consume a string when saving

