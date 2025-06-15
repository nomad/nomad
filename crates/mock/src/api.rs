use ed::ByteOffset;
use ed::command::{CommandArgs, CommandCompletion};
use ed::notify::Name;

use crate::value::{Map, Value};

/// TODO: docs.
#[derive(Default)]
pub struct Api {
    map: Map,
}

impl ed::Api for Api {
    type Value = Value;

    #[track_caller]
    fn add_constant(&mut self, constant_name: Name, value: Self::Value) {
        assert!(!self.map.contains_key(constant_name));
        self.map.insert(constant_name, value);
    }

    #[track_caller]
    fn add_function<Fun>(&mut self, function_name: Name, mut function: Fun)
    where
        Fun: FnMut(Self::Value) -> Option<Self::Value> + 'static,
    {
        assert!(!self.map.contains_key(function_name));
        let value = Value::Function(Box::new(move |value| {
            function(value).unwrap_or_default()
        }));
        self.map.insert(function_name, value);
    }

    #[track_caller]
    fn add_submodule(&mut self, module_name: Name, module_api: Self) {
        assert!(!self.map.contains_key(module_name));
        let value = Value::Map(module_api.map);
        self.map.insert(module_name, value);
    }

    fn add_command<Command, CompletionFn, Completions>(
        &mut self,
        _: Name,
        _: Command,
        _: CompletionFn,
    ) where
        Command: FnMut(CommandArgs) + 'static,
        CompletionFn: FnMut(CommandArgs, ByteOffset) -> Completions + 'static,
        Completions: IntoIterator<Item = CommandCompletion>,
    {
    }

    fn new(_: Name) -> Self {
        Self::default()
    }
}
