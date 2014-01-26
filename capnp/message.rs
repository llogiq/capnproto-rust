/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use common::*;
use arena::*;
use layout;

pub struct ReaderOptions {
    traversalLimitInWords : u64,
    nestingLimit : uint
}

pub static DEFAULT_READER_OPTIONS : ReaderOptions =
    ReaderOptions { traversalLimitInWords : 8 * 1024 * 1024, nestingLimit : 64 };


type SegmentId = u32;

pub trait MessageReader {
    fn get_segment<'a>(&'a self, id : uint) -> &'a [Word];
    fn arena<'a>(&'a self) -> &'a ReaderArena;
    fn get_options<'a>(&'a self) -> &'a ReaderOptions;
    fn get_root<'a, T : layout::FromStructReader<'a>>(&'a self) -> T {
        unsafe {
            let segment : *SegmentReader = std::ptr::to_unsafe_ptr(&self.arena().segment0);

            let pointer_reader = layout::PointerReader::get_root::<'a>(
                segment, (*segment).get_start_ptr(), self.get_options().nestingLimit as int);

            let result : T = layout::FromStructReader::from_struct_reader(
                pointer_reader.get_struct::<'a>(std::ptr::null()));

            result
        }

    }
}

pub struct SegmentArrayMessageReader<'a> {
    priv segments : &'a [ &'a [Word]],
    priv options : ReaderOptions,
    priv arena : ~ReaderArena
}


impl <'a> MessageReader for SegmentArrayMessageReader<'a> {
    fn get_segment<'b>(&'b self, id : uint) -> &'b [Word] {
        self.segments[id]
    }

    fn arena<'b>(&'b self) -> &'b ReaderArena {
        &*self.arena
    }

    fn get_options<'b>(&'b self) -> &'b ReaderOptions {
        return &self.options;
    }
}

impl <'a> SegmentArrayMessageReader<'a> {

    pub fn new<'b>(segments : &'b [&'b [Word]], options : ReaderOptions) -> SegmentArrayMessageReader<'b> {
        assert!(segments.len() > 0);
        SegmentArrayMessageReader {
            segments : segments,
            arena : ReaderArena::new(segments),
            options : options
        }
    }
}

pub enum AllocationStrategy {
    FIXED_SIZE,
    GROW_HEURISTICALLY
}

pub static SUGGESTED_FIRST_SEGMENT_WORDS : uint = 1024;
pub static SUGGESTED_ALLOCATION_STRATEGY : AllocationStrategy = GROW_HEURISTICALLY;

pub trait MessageBuilder {
    fn arena<'a>(&'a mut self) -> &'a mut BuilderArena;

    fn init_root<'a, T : layout::FromStructBuilder<'a> + layout::HasStructSize>(&'a mut self) -> T {
        let rootSegment = std::ptr::to_mut_unsafe_ptr(&mut self.arena().segment0);

        match self.arena().segment0.allocate(WORDS_PER_POINTER) {
            None => {fail!("could not allocate root pointer") }
            Some(location) => {
                //assert!(location == 0,
                //        "First allocated word of new segment was not at offset 0");

                let pb = layout::PointerBuilder::get_root(rootSegment, location);

                return layout::FromStructBuilder::from_struct_builder(
                    pb.init_struct(layout::HasStructSize::struct_size(None::<T>)));
            }
        }

    }

    fn get_segments_for_output<T>(&mut self, cont : |&[&[Word]]| -> T) -> T {
        self.arena().get_segments_for_output(cont)
    }


}

pub struct MallocMessageBuilder {
    priv arena : ~BuilderArena,
}

impl Drop for MallocMessageBuilder {
    fn drop(&mut self) { }
}

impl MallocMessageBuilder {

    pub fn new(first_segment_size : uint, allocationStrategy : AllocationStrategy) -> MallocMessageBuilder {
        let arena = BuilderArena::new(allocationStrategy, NumWords(first_segment_size));

        MallocMessageBuilder { arena : arena }
    }

    pub fn new_default() -> MallocMessageBuilder {
        MallocMessageBuilder::new(SUGGESTED_FIRST_SEGMENT_WORDS, SUGGESTED_ALLOCATION_STRATEGY)
    }

}

impl MessageBuilder for MallocMessageBuilder {
    fn arena<'a>(&'a mut self) -> &'a mut BuilderArena {
        &mut *self.arena
    }

}


pub struct ScratchSpaceMallocMessageBuilder<'a> {
    priv arena : ~BuilderArena,
    priv scratch_space : &'a mut [Word],
}


// TODO: figure out why rust thinks this is unsafe.
#[unsafe_destructor]
impl <'a> Drop for ScratchSpaceMallocMessageBuilder<'a> {
    fn drop(&mut self) {
        unsafe {
            let len = self.scratch_space.len();
            std::ptr::zero_memory(self.scratch_space.as_mut_ptr(), len);
        }
    }
}


impl <'a> ScratchSpaceMallocMessageBuilder<'a> {

    pub fn new<'b>(scratch_space : &'b mut [Word], allocationStrategy : AllocationStrategy)
               -> ScratchSpaceMallocMessageBuilder<'b> {
        let arena = BuilderArena::new(allocationStrategy, ZeroedWords(scratch_space));

        ScratchSpaceMallocMessageBuilder { arena : arena, scratch_space : scratch_space }
    }

    pub fn new_default<'b>(scratch_space : &'b mut [Word]) -> ScratchSpaceMallocMessageBuilder<'b> {
        ScratchSpaceMallocMessageBuilder::new(scratch_space, SUGGESTED_ALLOCATION_STRATEGY)
    }

}

impl <'a> MessageBuilder for ScratchSpaceMallocMessageBuilder<'a> {
    fn arena<'a>(&'a mut self) -> &'a mut BuilderArena {
        &mut *self.arena
    }
}
