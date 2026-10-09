#![no_std]

pub trait Operation {
    type Output;
}

pub enum Effect {
    TextInfer(TextInference),
}

pub struct TextInference {}

impl Operation for TextInference {
    type Output = usize;
}

pub struct Completion<O: Operation> {
    data: O::Output,
}
