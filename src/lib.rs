pub use mapstruct_derive::MapStruct;

#[cfg(test)]
mod test {
    use super::*;

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

    impl<'a> Into<Y<'a>> for &'a X {
        fn into(self) -> Y<'a> {
            Y {
                id: self.id,
                name: &*self.name,
                age: self.age,
                some: &*self.some,
                last_name: &*self.name,
            }
        }
    }

    #[test]
    fn test() {
        println!("Hello, world!")
    }
}
