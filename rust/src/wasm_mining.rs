//! WebAssembly mining worker for browser/iPhone nodes
//! Allows users to mine by simply staying on the website

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Array};
use web_sys::{console, Worker, WorkerOptions, MessageEvent};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Mining worker state
#[wasm_bindgen]
pub struct MiningWorker {
    is_mining: Arc<Mutex<bool>>,
    hashes_per_second: Arc<Mutex<f64>>,
    total_hashes: Arc<Mutex<u64>>,
}

#[wasm_bindgen]
impl MiningWorker {
    /// Create a new mining worker
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console::log_1(&"Mining worker initialized".into());
        MiningWorker {
            is_mining: Arc::new(Mutex::new(false)),
            hashes_per_second: Arc::new(Mutex::new(0.0)),
            total_hashes: Arc::new(Mutex::new(0.0)),
        }
    }
    
    /// Start mining
    #[wasm_bindgen]
    pub fn start_mining(&self) -> Promise {
        let is_mining = self.is_mining.clone();
        let hashes_per_second = self.hashes_per_second.clone();
        let total_hashes = self.total_hashes.clone();
        
        let promise = future_to_promise(async move {
            *is_mining.lock().unwrap() = true;
            console::log_1(&"Mining started".into());
            
            let mut hash_count = 0u64;
            let start_time = js_sys::Date::now();
            
            // Simulate mining work (in real implementation, this would do actual proof-of-work)
            while *is_mining.lock().unwrap() {
                // Simulate hash computation
                let _hash = format!("{}{}", hash_count, "nonce");
                hash_count += 1;
                
                *total_hashes.lock().unwrap() = hash_count;
                
                // Calculate hashes per second
                let elapsed = (js_sys::Date::now() - start_time) / 1000.0;
                if elapsed > 0.0 {
                    *hashes_per_second.lock().unwrap() = hash_count as f64 / elapsed;
                }
                
                // Yield to browser (simulate async work)
                wasm_bindgen_futures::JsFuture::from(
                    js_sys::Promise::new(&mut |resolve, _| {
                        let callback = Box::new(move || {
                            resolve.call0(&js_sys::Value::NULL).unwrap();
                        });
                        let callback = js_sys::Function::from(callback);
                        web_sys::window()
                            .unwrap()
                            .set_timeout_with_callback(&callback, 0)
                            .unwrap();
                    })
                ).await.unwrap();
            }
            
            JsValue::from_str("Mining stopped")
        });
        
        promise
    }
    
    /// Stop mining
    #[wasm_bindgen]
    pub fn stop_mining(&self) {
        *self.is_mining.lock().unwrap() = false;
        console::log_1(&"Mining stopped".into());
    }
    
    /// Get current mining status
    #[wasm_bindgen]
    pub fn get_status(&self) -> JsValue {
        let status = js_sys::Object::new();
        
        let is_mining = *self.is_mining.lock().unwrap();
        js_sys::Reflect::set(
            &status,
            &JsValue::from_str("is_mining"),
            &JsValue::from_bool(is_mining),
        ).unwrap();
        
        let hps = *self.hashes_per_second.lock().unwrap();
        js_sys::Reflect::set(
            &status,
            &JsValue::from_str("hashes_per_second"),
            &JsValue::from_f64(hps),
        ).unwrap();
        
        let total = *self.total_hashes.lock().unwrap();
        js_sys::Reflect::set(
            &status,
            &JsValue::from_str("total_hashes"),
            &JsValue::from_f64(total as f64),
        ).unwrap();
        
        status
    }
    
    /// Check if currently mining
    #[wasm_bindgen]
    pub fn is_mining(&self) -> bool {
        *self.is_mining.lock().unwrap()
    }
    
    /// Get hashes per second
    #[wasm_bindgen]
    pub fn get_hashes_per_second(&self) -> f64 {
        *self.hashes_per_second.lock().unwrap()
    }
    
    /// Get total hashes computed
    #[wasm_bindgen]
    pub fn get_total_hashes(&self) -> u64 {
        *self.total_hashes.lock().unwrap()
    }
}

/// Browser node that can mine while user stays on website
#[wasm_bindgen]
pub struct BrowserNode {
    mining_worker: MiningWorker,
    connected: Arc<Mutex<bool>>,
}

#[wasm_bindgen]
impl BrowserNode {
    /// Create a new browser node
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console::log_1(&"Browser node initialized".into());
        BrowserNode {
            mining_worker: MiningWorker::new(),
            connected: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Connect to the network
    #[wasm_bindgen]
    pub fn connect(&mut self) -> Promise {
        let connected = self.connected.clone();
        
        let promise = future_to_promise(async move {
            // Simulate network connection
            wasm_bindgen_futures::JsFuture::from(
                js_sys::Promise::new(&mut |resolve, _| {
                    let callback = Box::new(move || {
                        resolve.call0(&js_sys::Value::NULL).unwrap();
                    });
                    let callback = js_sys::Function::from(callback);
                    web_sys::window()
                        .unwrap()
                        .set_timeout_with_callback(&callback, 1000)
                        .unwrap();
                })
            ).await.unwrap();
            
            *connected.lock().unwrap() = true;
            console::log_1(&"Connected to network".into());
            
            JsValue::from_str("Connected")
        });
        
        promise
    }
    
    /// Start mining (contribute to network)
    #[wasm_bindgen]
    pub fn start_mining(&self) -> Promise {
        self.mining_worker.start_mining()
    }
    
    /// Stop mining
    #[wasm_bindgen]
    pub fn stop_mining(&self) {
        self.mining_worker.stop_mining()
    }
    
    /// Get node status
    #[wasm_bindgen]
    pub fn get_status(&self) -> JsValue {
        let status = js_sys::Object::new();
        
        let connected = *self.connected.lock().unwrap();
        js_sys::Reflect::set(
            &status,
            &JsValue::from_str("connected"),
            &JsValue::from_bool(connected),
        ).unwrap();
        
        let mining_status = self.mining_worker.get_status();
        js_sys::Reflect::set(
            &status,
            &JsValue::from_str("mining"),
            &mining_status,
        ).unwrap();
        
        status
    }
}

/// Utility function to log to browser console
#[wasm_bindgen]
pub fn log_message(message: &str) {
    console::log_1(&JsValue::from_str(message));
}

/// Get browser info
#[wasm_bindgen]
pub fn get_browser_info() -> JsValue {
    let info = js_sys::Object::new();
    
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        
        if let Some(user_agent) = navigator.user_agent() {
            js_sys::Reflect::set(
                &info,
                &JsValue::from_str("user_agent"),
                &JsValue::from_str(&user_agent),
            ).unwrap();
        }
        
        let screen = window.screen();
        js_sys::Reflect::set(
            &info,
            &JsValue::from_str("screen_width"),
            &JsValue::from_f64(screen.width() as f64),
        ).unwrap();
        
        js_sys::Reflect::set(
            &info,
            &JsValue::from_str("screen_height"),
            &JsValue::from_f64(screen.height() as f64),
        ).unwrap();
    }
    
    info
}
