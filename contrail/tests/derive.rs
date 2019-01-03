// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[macro_use]
extern crate contrail;

use contrail::mem::Bytes;

#[derive(Bytes, Clone, Copy, Debug, Eq, PartialEq)]
struct Foo {
    a: i64,
    b: [u8; 3],
}

#[test]
fn custom_derive() {
    let foo = Foo {
        a: 1234567891011,
        b: [1, 5, 3],
    };
    let mut bytes = [0u8; std::mem::size_of::<Foo>()];
    unsafe { foo.write_bytes(&mut bytes) };
    assert_eq!(unsafe { Foo::read_bytes(&bytes) }, foo);
}
