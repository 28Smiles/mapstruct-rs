# Mapstruct-Rs

A proc macro that generates new structs with the variations of the fields you want. You use it by annotating
a struct with `#[derive(Mapstruct)]` and then you can use the `mapstruct(...)` macro to generate new structs.
The syntax is similar to the struct definition syntax, but you only specify the fields and generics you want to change.
You use the `+` operator to add a field or generic, the `-` operator to remove a field or generic and the `~` operator
to change the type or name of a field or generic.

## Example Struct

```rust
use mapstruct_rs::Mapstruct;

#[derive(MapStruct)]
#[mapstruct(
    #[derive(Debug)]
    struct Y<
        +'a,
    > {
        ~id -> pub id,
        ~name: &'a str,
        ~some: &'a str,
        +last_name: &'a str,
        -height,
    }
)]
struct X {
    id: i64,
    name: String,
    age: i32,
    height: f32,
    some: String,
}

```
The above code will generate the following struct:
```rust
#[derive(Debug)]
struct Y<'a> {
    pub id: i64,
    name: &'a str,
    age: i32,
    some: &'a str,
    last_name: &'a str,
}
```

## Example Enum

```rust
use mapstruct_rs::Mapstruct;

#[derive(MapStruct)]
#[mapstruct(
    #[derive(Debug)]
    enum Y {
        A {
            id: i64,
        },
        ~B(~i32, _),
        +D(i8),
    }
)]
enum X {
    A(i64),
    B(i32, i8),
    C(i16),
}
```
The above code will generate the following enum:
```rust
#[derive(Debug)]
enum Y {
    A {
        id: i64,
    },
    B(i32),
    C(i16),
    D(i8)
}
```
