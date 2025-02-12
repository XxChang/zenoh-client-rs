//
// Copyright (c) 2023 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//

//! # Init message
//!
//! The INIT message is sent on a specific Locator to initiate a transport with the zenoh node
//! associated with that Locator. The initiator MUST send an INIT message with the A flag set to 0.
//! If the corresponding zenohd node deems appropriate to accept the INIT message, the corresponding
//! peer MUST reply with an INIT message with the A flag set to 1. Alternatively, it MAY reply with
//! a [`super::Close`] message. For convenience, we call [`InitSyn`] and [`InitAck`] an INIT message
//! when the A flag is set to 0 and 1, respectively.
//!
//! The [`InitSyn`]/[`InitAck`] message flow is the following:
//!
//! ```text
//!     A                   B
//!     |      INIT SYN     |
//!     |------------------>|
//!     |                   |
//!     |      INIT ACK     |
//!     |<------------------|
//!     |                   |
//! ```
//!
//! The INIT message structure is defined as follows:
//!
//! ```text
//! Flags:
//! - A: Ack            If A==0 then the message is an InitSyn else it is an InitAck
//! - S: Size params    If S==1 then size parameters are exchanged
//! - Z: Extensions     If Z==1 then zenoh extensions will follow.
//!
//!  7 6 5 4 3 2 1 0
//! +-+-+-+-+-+-+-+-+
//! |Z|S|A|   INIT  |
//! +-+-+-+---------+
//! |    version    |
//! +---------------+
//! |zid_len|x|x|wai| (#)(*)
//! +-------+-+-+---+
//! ~      [u8]     ~ -- ZenohID of the sender of the INIT message
//! +---------------+
//! |x|x|x|x|rid|fsn| \                -- SN/ID resolution (+)
//! +---------------+  | if Flag(S)==1
//! |      u16      |  |               -- Batch Size ($)
//! |               | /
//! +---------------+
//! ~    <u8;z16>   ~ -- if Flag(A)==1 -- Cookie
//! +---------------+
//! ~   [InitExts]  ~ -- if Flag(Z)==1
//! +---------------+
//!
//! If A==1 and S==0 then size parameters are (ie. S flag) are accepted.
//!
//! (*) WhatAmI. It indicates the role of the zenoh node sending the INIT message.
//!    The valid WhatAmI values are:
//!    - 0b00: Router
//!    - 0b01: Peer
//!    - 0b10: Client
//!    - 0b11: Reserved
//!
//! (#) ZID length. It indicates how many bytes are used for the ZenohID bytes.
//!     A ZenohID is minimum 1 byte and maximum 16 bytes. Therefore, the actual length is computed as:
//!         real_zid_len := 1 + zid_len
//!
//! (+) Sequence Number/ID resolution. It indicates the resolution and consequently the wire overhead
//!     of various SN and ID in Zenoh.
//!     - fsn: frame/fragment sequence number resolution. Used in Frame/Fragment messages.
//!     - rid: request ID resolution. Used in Request/Response messages.
//!     The valid SN/ID resolution values are:
//!     - 0b00: 8 bits
//!     - 0b01: 16 bits
//!     - 0b10: 32 bits
//!     - 0b11: 64 bits
//!
//! ($) Batch Size. It indicates the maximum size of a batch the sender of the INIT message is willing
//!     to accept when reading from the network. Default on unicast: 65535.
//!
//! NOTE: 16 bits (2 bytes) may be prepended to the serialized message indicating the total length
//!       in bytes of the message, resulting in the maximum length of a message being 65535 bytes.
//!       This is necessary in those stream-oriented transports (e.g., TCP) that do not preserve
//!       the boundary of the serialized messages. The length is encoded as little-endian.
//!       In any case, the length of a message must not exceed 65535 bytes.
//! ```

pub mod flag {
    pub const A: u8 = 1 << 5; // 0x20 Ack           if A==0 then the message is an InitSyn else it is an InitAck
    pub const S: u8 = 1 << 6; // 0x40 Size params   if S==1 then size parameters are exchanged
    pub const Z: u8 = 1 << 7; // 0x80 Extensions    if Z==1 then an extension will follow
}

