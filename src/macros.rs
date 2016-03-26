macro_rules! index {
    (pub struct $name:ident;) => {
        #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name {
            u32_index: u32
        }

        impl From<usize> for $name {
            fn from(index: usize) -> $name {
                assert!(index < (::std::u32::MAX - 1) as usize);
                $name {
                    u32_index: index as u32 + 1
                }
            }
        }

        impl $name {
            pub fn index(&self) -> usize {
                (self.u32_index as usize) - 1
            }
        }
    }
}

macro_rules! indexed_vec {
    (pub struct $name:ident<$elem:ty>[$index:ty];) => {
        pub struct $name {
            data: Vec<$elem>
        }

        impl From<Vec<$elem>> for $name {
            pub fn from(data: Vec<$elem>) -> $name {
                $name { data: data }
            }
        }

        impl $name {
            pub fn new() -> $name {
                $name { data: vec![] }
            }
        }

        impl ::std::ops::Index<$index> for $name {
            type Output = $elem;

            fn index(&self, index: $index) -> &Self::Output {
                &self.data[index.index()]
            }
        }

        impl ::std::ops::IndexMut<$index> for $name {
            type Output = $elem;

            fn index_mut(&mut self, index: $index) -> &mut Self::Output {
                &mut self.data[index.index()]
            }
        }
    }
}
