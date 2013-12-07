/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use io;
use message::*;
use serialize::*;

#[inline]
unsafe fn ptr_sub<T, U:std::ptr::RawPtr<T>>(p1 : U, p2 : U) -> uint {
    return (p1.to_uint() - p2.to_uint()) / std::mem::size_of::<T>();
}

pub struct PackedInputStream<'a, 'b,  R> {
    inner : &'a mut io::BufferedInputStream<'b, R>
}

macro_rules! refresh_buffer(
    ($inner:expr, $size:ident, $inPtr:ident, $inEnd:ident, $out:ident,
     $outBuf:ident, $bufferBegin:ident) => (
        {
            $inner.skip($size);
            let (b, e) = $inner.get_read_buffer();
            $inPtr = b;
            $inEnd = e;
            $size = ptr_sub($inEnd, $inPtr);
            $bufferBegin = b;
            assert!($size > 0);
        }
        );
    )

impl <'a, 'b, R : std::io::Reader> std::io::Reader for PackedInputStream<'a, 'b, R> {
    fn eof(&mut self) -> bool {
        self.inner.eof()
    }

    fn read(&mut self, outBuf: &mut [u8]) -> Option<uint> {
        let len = outBuf.len();

        if (len == 0) { return Some(0); }

        assert!(len % 8 == 0, "PackInputStream reads must be word-aligned");

        unsafe {
            let mut out = outBuf.unsafe_mut_ref(0);
            let outEnd = outBuf.unsafe_mut_ref(len);

            let (mut inPtr, mut inEnd) = self.inner.get_read_buffer();
            let mut bufferBegin = inPtr;
            let mut size = ptr_sub(inEnd, inPtr);
            if size == 0 {
                return Some(0);
            }

            loop {

                let mut tag : u8;

                assert!(ptr_sub(out, outBuf.unsafe_mut_ref(0)) % 8 == 0,
                        "Output pointer should always be aligned here.");

                if ptr_sub(inEnd, inPtr) < 10 {
                    if out >= outEnd {
                        self.inner.skip(ptr_sub(inPtr, bufferBegin));
                        return Some(ptr_sub(out, outBuf.unsafe_mut_ref(0)));
                    }

                    if ptr_sub(inEnd, inPtr) == 0 {
                        refresh_buffer!(self.inner, size, inPtr, inEnd, out, outBuf, bufferBegin);
                        continue;
                    }

                    //# We have at least 1, but not 10, bytes available. We need to read
                    //# slowly, doing a bounds check on each byte.

                    tag = *inPtr;
                    inPtr = inPtr.offset(1);

                    for i in range(0, 8) {
                        if (tag & (1u8 << i)) != 0 {
                            if ptr_sub(inEnd, inPtr) == 0 {
                                refresh_buffer!(self.inner, size, inPtr, inEnd,
                                                out, outBuf, bufferBegin);
                            }
                            *out = *inPtr;
                            out = out.offset(1);
                            inPtr = inPtr.offset(1);
                        } else {
                            *out = 0;
                            out = out.offset(1);
                        }
                    }

                    if ptr_sub(inEnd, inPtr) == 0 && (tag == 0 || tag == 0xff) {
                        refresh_buffer!(self.inner, size, inPtr, inEnd,
                                        out, outBuf, bufferBegin);
                    }
                } else {
                    tag = *inPtr;
                    inPtr = inPtr.offset(1);

                    for n in range(0, 8) {
                        let isNonzero = (tag & (1 as u8 << n)) != 0;
                        *out = (*inPtr) & ((-(isNonzero as i8)) as u8);
                        out = out.offset(1);
                        inPtr = inPtr.offset(isNonzero as int);
                    }
                }
                if (tag == 0) {
                    assert!(ptr_sub(inEnd, inPtr) > 0,
                            "Should always have non-empty buffer here");

                    let runLength : uint = (*inPtr) as uint * 8;
                    inPtr = inPtr.offset(1);

                    assert!(runLength <= ptr_sub(outEnd, out),
                            "Packed input did not end cleanly on a segment boundary");

                    std::ptr::set_memory(out, 0, runLength);
                    out = out.offset(runLength as int);

                } else if (tag == 0xff) {
                    assert!(ptr_sub(inEnd, inPtr) > 0,
                            "Should always have non-empty buffer here");

                    let mut runLength : uint = (*inPtr) as uint * 8;
                    inPtr = inPtr.offset(1);

                    assert!(runLength <= ptr_sub(outEnd, out),
                            "Packed input did not end cleanly on a segment boundary");

                    let inRemaining = ptr_sub(inEnd, inPtr);
                    if (inRemaining >= runLength) {
                        //# Fast path.
                        std::ptr::copy_nonoverlapping_memory(out, inPtr, runLength);
                        out = out.offset(runLength as int);
                        inPtr = inPtr.offset(runLength as int);
                    } else {
                        //# Copy over the first buffer, then do one big read for the rest.
                        std::ptr::copy_nonoverlapping_memory(out, inPtr, inRemaining);
                        out = out.offset(inRemaining as int);
                        runLength -= inRemaining;

                        self.inner.skip(size);
                        std::vec::raw::mut_buf_as_slice::<u8,()>(out, runLength, |buf| {
                            self.inner.read(buf);
                        });
                        out = out.offset(runLength as int);

                        if (out == outEnd) {
                            return Some(len);
                        } else {
                            let (b, e) = self.inner.get_read_buffer();
                            inPtr = b;
                            inEnd = e;
                            size = ptr_sub(e, b);
                            bufferBegin = inPtr;
                            continue;
                        }
                    }
                }

                if (out == outEnd) {
                    self.inner.skip(ptr_sub(inPtr, bufferBegin));
                    return Some(len);
                }
            }
        }
    }
}

pub struct PackedOutputStream<'a, 'b, W> {
    inner : &'a mut io::BufferedOutputStream<'b, W>
}

impl <'a, 'b, W : std::io::Writer> std::io::Writer for PackedOutputStream<'a, 'b, W> {
    fn write(&mut self, inBuf : &[u8]) {
        unsafe {
            let (mut out, mut bufferEnd) = self.inner.get_write_buffer();
            let mut bufferBegin = out;
            let mut slowBuffer : [u8,..20] = [0, ..20];

            let mut inPtr : *u8 = inBuf.unsafe_ref(0);
            let inEnd : *u8 = inBuf.unsafe_ref(inBuf.len());

            while (inPtr < inEnd) {

                if (ptr_sub(bufferEnd, out) < 10) {
                    //# Oops, we're out of space. We need at least 10
                    //# bytes for the fast path, since we don't
                    //# bounds-check on every byte.
                    self.inner.write_ptr(bufferBegin, ptr_sub(out, bufferBegin));

                    out = slowBuffer.unsafe_mut_ref(0);
                    bufferEnd = slowBuffer.unsafe_mut_ref(20);
                    bufferBegin = out;
                }

                let tagPos : *mut u8 = out;
                out = out.offset(1);

                let bit0 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit0 as int);
                inPtr = inPtr.offset(1);

                let bit1 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit1 as int);
                inPtr = inPtr.offset(1);

                let bit2 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit2 as int);
                inPtr = inPtr.offset(1);

                let bit3 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit3 as int);
                inPtr = inPtr.offset(1);

                let bit4 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit4 as int);
                inPtr = inPtr.offset(1);

                let bit5 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit5 as int);
                inPtr = inPtr.offset(1);

                let bit6 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit6 as int);
                inPtr = inPtr.offset(1);

                let bit7 = (*inPtr != 0) as u8;
                *out = *inPtr;
                out = out.offset(bit7 as int);
                inPtr = inPtr.offset(1);


                let tag : u8 = (bit0 << 0) | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
                    | (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7);

                *tagPos = tag;

                if (tag == 0) {
                    //# An all-zero word is followed by a count of
                    //# consecutive zero words (not including the first
                    //# one).

                    let mut inWord : *u64 = std::cast::transmute(inPtr);
                    let mut limit : *u64 = std::cast::transmute(inEnd);
                    if (ptr_sub(limit, inWord) > 255) {
                        limit = inWord.offset(255);
                    }
                    while (inWord < limit && *inWord == 0) {
                        inWord = inWord.offset(1);
                    }
                    *out = ptr_sub(inWord, std::cast::transmute::<*u8, *u64>(inPtr)) as u8;

                    out = out.offset(1);
                    inPtr = std::cast::transmute::<*u64, *u8>(inWord);

                } else if (tag == 0xff) {
                    //# An all-nonzero word is followed by a count of
                    //# consecutive uncompressed words, followed by the
                    //# uncompressed words themselves.

                    //# Count the number of consecutive words in the input
                    //# which have no more than a single zero-byte. We look
                    //# for at least two zeros because that's the point
                    //# where our compression scheme becomes a net win.
                    let runStart = inPtr;
                    let mut limit = inEnd;
                    if (ptr_sub(limit, inPtr) > 255 * 8) {
                        limit = inPtr.offset(255 * 8);
                    }

                    while (inPtr < limit) {
                        let mut c = 0;

                        for _ in range(0,8) {
                            c += (*inPtr == 0) as u8;
                            inPtr = inPtr.offset(1);
                        }

                        if (c >= 2) {
                            //# Un-read the word with multiple zeros, since
                            //# we'll want to compress that one.
                            inPtr = inPtr.offset(-8);
                            break;
                        }
                    }
                    let count : uint = ptr_sub(inPtr, runStart);
                    *out = (count / 8) as u8;
                    out = out.offset(1);

                    if (count <= ptr_sub(bufferEnd, out)) {
                        //# There's enough space to memcpy.

                        let src : *u8 = runStart;
                        std::ptr::copy_nonoverlapping_memory(out, src, count);

                        out = out.offset(count as int);
                    } else {
                        //# Input overruns the output buffer. We'll give it
                        //# to the output stream in one chunk and let it
                        //# decide what to do.
                        self.inner.write_ptr(bufferBegin, ptr_sub(out, bufferBegin));

                        std::vec::raw::buf_as_slice::<u8,()>(runStart, count, |buf| {
                            self.inner.write(buf);
                        });

                        let (out1, bufferEnd1) = self.inner.get_write_buffer();
                        out = out1; bufferEnd = bufferEnd1;
                        bufferBegin = out;
                    }
                }
            }

            self.inner.write_ptr(bufferBegin, ptr_sub(out, bufferBegin));
        }
    }

   fn flush(&mut self) { self.inner.flush(); }
}

pub trait WritePacked {
    fn write_packed_message(&mut self, message : &MessageBuilder);
}

impl <'a, T : std::io::Writer> WritePacked for io::BufferedOutputStream<'a, T> {
    fn write_packed_message(&mut self, message : &MessageBuilder) {
        let mut packedOutputStream = PackedOutputStream {inner : self};
        write_message(&mut packedOutputStream, message);
    }
}

pub struct WritePackedWrapper<'a, T> {writer : &'a mut T }

impl <'a, T: std::io::Writer> WritePacked for WritePackedWrapper<'a, T> {
    fn write_packed_message(&mut self, message : &MessageBuilder) {
        let mut buffered = io::BufferedOutputStream::new(self.writer);
        buffered.write_packed_message(message);
        buffered.flush();
    }
}