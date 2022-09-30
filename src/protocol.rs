use crate::connection::Conn;

#[derive(Debug)]
pub enum ParseHint {
    Complete(usize),
    Incomplete(usize),
    Invalid,
}

macro_rules! protocol {
    (
        enum $enum_name:ident {
            $(
                $variant_name:ident$(($($member:ty),*) match as ($($mid:ident),*))?
            ),+
        }
    ) => {
        mod discriminant {
            pub enum $enum_name {
                $(
                    $variant_name
                ),+
            }
        }

        #[derive(Debug)]
        pub enum $enum_name {
            $(
                $variant_name$(($($member),*))?
            ),+
        }

        impl Serial for $enum_name {
            fn serialize(&self) -> Result<Vec<u8>, ()> {
                match self {
                    $(
                        $enum_name::$variant_name $(( $($mid),* ))? => {
                            let buffer = vec![discriminant::$enum_name::$variant_name as u8];
                            $(
                            let mut buffer = buffer;
                            $(
                                buffer.append(&mut $mid.serialize()?);
                            )*
                            )?
                            Ok(buffer)
                        }
                    )+
                }
            }
            fn deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), ()> where Self: Sized {
                let hint = Self::parse_hint(bytes);
                match hint {
                    ParseHint::Complete(_) => (),
                    _ => return Err(())
                }
                let discriminant = bytes[0];
                let mut bytes = &bytes[1..];
                match discriminant {
                    $(
                        d if d == discriminant::$enum_name::$variant_name as u8 => {
                            Ok(($enum_name::$variant_name$(
                            ($(
                                {
                                    let (member, byte_tail) = <$member>::deserialize(bytes)?;
                                    bytes = &byte_tail;
                                    member
                                }
                            ),*)
                            )?, bytes))
                        }
                    )+
                    _ => unreachable!()
                }
            }
            fn minimum_size() -> usize {
                1
            }
            #[allow(unused_assignments)]
            fn parse_hint(bytes: &[u8]) -> ParseHint {
                if bytes.len() <= 0 {
                    return ParseHint::Incomplete(1);
                }
                let discriminant = bytes[0];
                let mut bytes = &bytes[1..];
                match discriminant {
                    $(
                        d if d == discriminant::$enum_name::$variant_name as u8 => {
                            $(
                                let total_minimum_size = 0 $( + <$member>::minimum_size() )*;
                                let mut missing_bytes = total_minimum_size;
                            )?
                            let bytes_consumed = 1;
                            $(
                            let mut bytes_consumed = bytes_consumed;
                            $(
                                let required_bytes = <$member>::minimum_size();
                                if required_bytes > bytes.len() {
                                    return ParseHint::Incomplete(missing_bytes - bytes.len())
                                } else {
                                    missing_bytes -= required_bytes;
                                    let hint = <$member>::parse_hint(bytes);
                                    match hint {
                                        ParseHint::Complete(consumed) => {
                                            bytes_consumed += consumed;
                                            bytes = &bytes[consumed..];
                                        }
                                        ParseHint::Incomplete(missing) => {
                                            return ParseHint::Incomplete(missing_bytes + missing)
                                        }
                                        hint => return hint
                                    }
                                }
                            )*
                            )?
                            return ParseHint::Complete(bytes_consumed)
                        }
                    )+
                    _ => return ParseHint::Invalid
                }
            }
        }

        pub type Connection = Conn<$enum_name>;
    };
}

pub trait Serial {
    fn serialize(&self) -> Result<Vec<u8>, ()>;
    fn deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), ()> where Self: Sized;
    fn minimum_size() -> usize;
    fn parse_hint(bytes: &[u8]) -> ParseHint;
}

type VecLength = u16;

macro_rules! serial_big_endian {
    ($int:ty) => {
        impl Serial for $int {
            fn serialize(&self) -> Result<Vec<u8>, ()> {
                Ok((&self.to_be_bytes()[..]).to_vec())
            }
            fn deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), ()> where Self: Sized {
                use std::convert::TryInto;
                const SIZE: usize = std::mem::size_of::<$int>();
                if bytes.len() < SIZE {
                    Err(())
                } else {
                    let be_bytes = bytes[..SIZE].try_into().unwrap();
                    Ok((Self::from_be_bytes(be_bytes), &bytes[SIZE..]))
                }
            }
            fn minimum_size() -> usize {
                std::mem::size_of::<$int>()
            }
            fn parse_hint(bytes: &[u8]) -> ParseHint {
                let size = Self::minimum_size();
                if size > bytes.len() {
                    ParseHint::Incomplete(size - bytes.len())
                } else {
                    ParseHint::Complete(size)
                }
            }
        }
        impl Serial for Vec<$int> {
            fn serialize(&self) -> Result<Vec<u8>, ()> {
                let len = self.len() * std::mem::size_of::<$int>();
                if len > u16::MAX as usize {
                    return Err(())
                } else {
                    let mut buffer = (len as u16).serialize()?;
                    for elem in self {
                        buffer.append(&mut elem.serialize()?)
                    }
                    Ok(buffer)
                }
            }
            fn deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), ()> where Self: Sized {
                let element_size = std::mem::size_of::<$int>();
                let (len, mut bytes) = VecLength::deserialize(bytes)?;
                let len = len as usize;
                if len > bytes.len() {
                    Err(())
                } else if len % element_size != 0 {
                    Err(())
                } else {
                    let element_len = len / element_size;
                    let mut buffer = Vec::new();
                    for _ in 0..element_len {
                        let (elem, bytes_tail) = <$int>::deserialize(bytes)?;
                        bytes = bytes_tail;
                        buffer.push(elem);
                    }
                    Ok((buffer, bytes))
                }
            }
            fn minimum_size() -> usize {
                std::mem::size_of::<VecLength>()
            }
            fn parse_hint(bytes: &[u8]) -> ParseHint {
                let min_size = Self::minimum_size();
                if min_size > bytes.len() {
                    ParseHint::Incomplete(min_size - bytes.len())
                } else if let Ok((len, bytes)) = VecLength::deserialize(bytes) {
                    let len = len as usize;
                    if len > bytes.len() {
                        ParseHint::Incomplete(len - bytes.len())
                    } else {
                        ParseHint::Complete(min_size + len)
                    }
                } else {
                    unreachable!()
                }
            }
        }
    };
    ($($int:ty),+) => {
        $(
            serial_big_endian!($int);
        )+
    }
}

serial_big_endian!(u8, u16, u32, u64, u128);
serial_big_endian!(i8, i16, i32, i64, i128);

protocol! {
    enum Event {
        Awaiting(u8) match as (players),
        See(u32, u32, u8) match as (x, y, char),
        Know(u32, u32, u8) match as (x, y, char),
        Hide(u32, u32) match as (x, y),
        HideAll,
        Forget(u32, u32) match as (x, y),
        ForgetAll,
        BombCount(u8) match as (count),
        Flush,
        Ping,
        KeyUp(u8) match as (key),
        KeyDown(u8) match as (key),
        Lose,
        Win
    }
}