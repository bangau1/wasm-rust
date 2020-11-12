// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::str;

use std::time::Duration;

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(proxy_wasm::types::LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(HttpAuthRootContext {
            config: FilterConfig {
                auth_cluster: "".to_string(),
                auth_host: "".to_string(),
            },
        })
    });
}

struct HttpAuth {
    config: FilterConfig,
}
struct HttpAuthRootContext {
    config: FilterConfig,
}
struct FilterConfig {
    auth_cluster: String,
    auth_host: String,
}

impl HttpAuth {
    fn fail(&mut self) {
        log::info!("auth: not allowed");
        self.send_http_response(403, vec![], Some(b"not authorized"));
    }
}

// Implement http functions related to this request.
// This is the core of the filter code.
impl HttpContext for HttpAuth {
    // This callback will be invoked when request headers arrive
    fn on_http_request_headers(&mut self, _: usize) -> Action {
        // get all the request headers
        let headers = self.get_http_request_headers();
        // log::info!("\nRequest Headers: \n{:?}\n\n", headers);
        // transform them from Vec<(String,String)> to Vec<(&str,&str)>; as dispatch_http_call needs
        // Vec<(&str,&str)>.
        // let ref_headers: Vec<(&str, &str)> = headers
        //     .iter()
        //     .map(|(ref k, ref v)| (k.as_str(), v.as_str()))
        //     .collect();

        // Dispatch a call to the auth-cluster. Here we assume that envoy's config has a cluster
        // named auth-cluster. We send the auth cluster all our headers, so it has context to
        // perform auth decisions.
        let res = self.dispatch_http_call(
            "auth-cluster", // cluster name
            vec![
                (":method", "GET"),
                (":path", "/"),
                (":authority", "www.google.com"),
            ], // headers
            None,           // no body
            vec![],         // no trailers
            Duration::from_secs(1), // one second timeout
        );

        // If dispatch reutrn an error, fail the request.
        match res {
            Err(status @ _) => {
                log::info!("Error when dispatch http call: {:?}", status);
                self.fail();
            }
            Ok(_) => {}
        }

        // the dispatch call is asynchronous. This means it returns immediatly, while the request
        // happens in the background. When the response arrives `on_http_call_response` will be
        // called. In the mean time, we need to pause the request, so it doesn't continue upstream.
        Action::Pause
    }

    fn on_http_response_headers(&mut self, _: usize) -> Action {
        log::info!(
            "response_header from upstream server: \n{:?}\n",
            self.get_http_response_headers()
        );

        // Add a header on the response.
        self.set_http_response_header("Hello", Some("world"));
        Action::Continue
    }
}

impl Context for HttpAuth {
    fn on_http_call_response(
        &mut self,
        _token_id: u32,
        _num_headers: usize,
        _body_size: usize,
        _num_trailers: usize,
    ) {
        // We have a response to the http call!
        log::info!("We got response back from auth-cluster");
        // log::info!("headers: {:?}", self.get_http_call_response_headers());
        // let body = self.get_http_call_response_body(0, _body_size);
        let headers = self.get_http_call_response_headers();
        let mut header_maps = HashMap::new();
        for (key, value) in headers {
            header_maps.insert(key, value);
        }

        // match body {
        //     Some(p) => log::info!("{:?}", str::from_utf8(&p)),
        //     None => log::info!("empty body"),
        // }

        // if we have no headers, it means the http call failed. Fail the incoming request as well.
        if _num_headers == 0 {
            log::info!("response header size is zero!");
            self.fail();
            return;
        }

        // Check if the auth server returned "200", if so call `resume_http_request` so request is
        // sent upstream.
        // Otherwise, fail the incoming request.
        match header_maps.get(":status") {
            Some(status) if status == "200" => {
                self.resume_http_request();
            }
            _ => {
                log::info!("auth: not authorized");
                self.fail();
            }
        }
    }
}

impl Context for HttpAuthRootContext {}
impl RootContext for HttpAuthRootContext {
    fn on_vm_start(&mut self, _vm_configuration_size: usize) -> bool {
        log::info!("VM STARTED");
        true
    }

    fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
        log::info!("READING CONFIG");

        if self.config.auth_cluster == "" {
            // log::info!("CONFIG EMPTY");
            if let Some(config_bytes) = self.get_configuration() {
                log::info!("GOT CONFIG");
                log::info!("config: {:?}", str::from_utf8(&config_bytes));
            // TODO: some proper error handling here
            // let cfg: Value = serde_json::from_slice(config_bytes.as_slice()).unwrap();
            // self.config.auth_cluster = cfg
            //     .get("auth_cluster")
            //     .unwrap()
            //     .as_str()
            //     .unwrap()
            //     .to_string();
            // self.config.auth_host = cfg.get("auth_host").unwrap().as_str().unwrap().to_string();
            } else {
                log::info!("NO CONFIG FOUND+++++++++++++++++==");
            }
        }
        true
    }
    fn create_http_context(&self, _context_id: u32, _root_context_id: u32) -> Box<dyn HttpContext> {
        Box::new(HttpAuth {
            config: FilterConfig {
                auth_cluster: self.config.auth_cluster.clone(),
                auth_host: self.config.auth_host.clone(),
            },
        })
    }
}
