use serde::Serialize;
use wasmer::{imports, Function, Instance, Module, Store, Value};

pub const SNAKE_RUNTIME_WASM: &[u8] = include_bytes!("snake_runtime.wasm");

pub struct SnakeRuntime {
    store: Store,
    red_ptr: i32,
    red_len: u32,
    blue_ptr: i32,
    blue_len: u32,
    run_game: Function,
    result_get_ticks: Function,
    result_get_reason_len: Function,
    result_get_reason_byte: Function,
    result_get_winner: Function,
    result_get_cycles: Function,
    result_drop: Function,
}

impl SnakeRuntime {
    pub fn new(red_wasm: &[u8], blue_wasm: &[u8]) -> Self {
        let mut store = Store::default();
        let module = Module::new(&store, &SNAKE_RUNTIME_WASM).unwrap();
        let import_object = imports! {};
        let instance = Instance::new(&mut store, &module, &import_object).unwrap();
        let allocate_bytes = instance.exports.get_function("allocate_bytes").unwrap();

        let red_ptr = allocate_bytes
            .call(&mut store, &[Value::I32(red_wasm.len() as i32)])
            .unwrap()[0]
            .i32()
            .unwrap();

        let blue_ptr = allocate_bytes
            .call(&mut store, &[Value::I32(blue_wasm.len() as i32)])
            .unwrap()[0]
            .i32()
            .unwrap();

        let memory = instance.exports.get_memory("memory").unwrap();
        let memory_view = memory.view(&store);
        memory_view.write(red_ptr as u64, red_wasm).unwrap();
        memory_view.write(blue_ptr as u64, blue_wasm).unwrap();

        SnakeRuntime {
            store,
            red_ptr,
            red_len: red_wasm.len() as u32,
            blue_ptr,
            blue_len: blue_wasm.len() as u32,
            run_game: instance.exports.get_function("run_game").unwrap().clone(),
            result_get_winner: instance
                .exports
                .get_function("result_get_winner")
                .unwrap()
                .clone(),
            result_get_ticks: instance
                .exports
                .get_function("result_get_ticks")
                .unwrap()
                .clone(),
            result_get_cycles: instance
                .exports
                .get_function("result_get_cycles")
                .unwrap()
                .clone(),
            result_get_reason_len: instance
                .exports
                .get_function("result_get_reason_len")
                .unwrap()
                .clone(),
            result_get_reason_byte: instance
                .exports
                .get_function("result_get_reason_byte")
                .unwrap()
                .clone(),
            result_drop: instance
                .exports
                .get_function("result_drop")
                .unwrap()
                .clone(),
        }
    }
}

impl SnakeRuntime {
    pub fn run_game(&mut self, seed: u32) -> GameResult {
        let result_ptr = self
            .run_game
            .call(
                &mut self.store,
                &[
                    Value::I32(self.red_ptr),
                    Value::I32(self.red_len as i32),
                    Value::I32(self.blue_ptr),
                    Value::I32(self.blue_len as i32),
                    Value::I32(seed as i32),
                ],
            )
            .unwrap()[0]
            .unwrap_i32();

        let reason_len = self
            .result_get_reason_len
            .call(&mut self.store, &[Value::I32(result_ptr)])
            .unwrap()[0]
            .i32()
            .unwrap();

        let mut reason_bytes = vec![];
        for i in 0..reason_len {
            let byte = self
                .result_get_reason_byte
                .call(&mut self.store, &[Value::I32(result_ptr), Value::I32(i)])
                .unwrap()[0]
                .i32()
                .unwrap();
            reason_bytes.push(byte as u8);
        }
        let reason = String::from_utf8_lossy(&reason_bytes);

        let winner_value = self
            .result_get_winner
            .call(&mut self.store, &[Value::I32(result_ptr)])
            .unwrap()[0]
            .i32()
            .unwrap();

        let winner = match winner_value {
            0 => Winner::Red,
            1 => Winner::Blue,
            2 => Winner::Tie,
            3 => {
                panic!("RED WASM failed validation");
            }
            4 => {
                panic!("BLUE WASM failed validation");
            }
            _ => unreachable!(),
        };

        let ticks = self
            .result_get_ticks
            .call(&mut self.store, &[Value::I32(result_ptr)])
            .unwrap()[0]
            .i32()
            .unwrap();
        let cycles = self
            .result_get_cycles
            .call(&mut self.store, &[Value::I32(result_ptr)])
            .unwrap()[0]
            .i32()
            .unwrap();

        self.result_drop
            .call(&mut self.store, &[Value::I32(result_ptr)])
            .unwrap();

        GameResult {
            winner,
            tick: ticks as u32,
            cycle: cycles as u32,
            lose_reason: reason.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct GameResult {
    pub winner: Winner,
    pub tick: u32,
    pub cycle: u32,
    pub lose_reason: String,
}

#[derive(Serialize, Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Winner {
    Red,
    Blue,
    Tie,
}
