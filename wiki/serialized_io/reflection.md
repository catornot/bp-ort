# reflection
serialized_io provides a reflection api for registered squirrel structs which
allows for those types to be serialized and deserialized. This is performed
by calling `BPRegisterType` on a certain type which will then register a few
register native squirrel functions for the rest of serialized_io's api just for
this type.

### `BPRegisterType`
`void functionref( string type )`

#### Input
The type which needs to be reflected.

### Errors
Will throw a script exception if the type could not get reflected.
Errors will generally not be thrown since they will only occur if serialized_io finds a type that can't be reflected.

a non exhaustive list of such types
- entity
- class
- userdata
- var
- table (untyped)
- array (untyped)
- missing structs references

### Usage
In your mod.json you need to add an init script with a call back to call `BPRegisterType`
```json
{
  "InitScript": {
    "InitScript": "your_init_script.gnut",
    "InitScriptCallback": "InitCallback",
  }
}
```
and in the init script
```c
global function InitCallback

global struct SerializableStruct {
    int a,
    string b, 
    array<string> c,
}

void function InitCallback() {
    BPRegisterType( "SerializableStruct" ) // registers the struct 
    BPRegisterType( "array< string >" ) // normal types also need to be registered
}
```

then the following types become available
- `BPSerialize<TypeName>`
- `BPDeserialize<TypeName>`
- `BPLoadFile<TypeName>`
- `BPLoadFileAsync<TypeName>`
- `BPSaveFile<TypeName>`
- `BPSaveFileAsync<TypeName>`

where `TypeName` is the registered type but in PascalCase

### `BPSerialize<TypeName>`
`string functionref( TypeName obj )`
Serializes the type to json.

#### Errors
Will throw a script error if the serialization wasn't successful.

### `BPDeserialize<TypeName>`
`TypeName functionref( string json )`
Deserializes the type from json.

#### Errors
Will throw a script error if the deserialization wasn't successful.

- Note: currently the this function can crash if the json doesn't follow the type schema tho this will be resolved soon (couldn't be outdated by the time you are reading this lol)
