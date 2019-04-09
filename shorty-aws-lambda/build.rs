// Copyright 2019 Federico Fissore
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

fn main() {
    let os = std::env::consts::OS;

    // according to https://medium.com/@kkostov/rust-aws-lambda-30a1b92d4009
    // on macos linker executable is named "x86_64-linux-musl-gcc" instead of "musl-gcc"
    if os == "macos" {
        std::env::set_var("RUSTC_LINKER", "x86_64-linux-musl-gcc");
    }
}
