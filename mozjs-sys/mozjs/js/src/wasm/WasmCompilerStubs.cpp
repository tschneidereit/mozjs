/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*-
 * vim: set ts=8 sts=2 et sw=2 tw=80:
 *
 * Copyright 2025 Mozilla Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// Stub implementations for the JIT-based wasm compilers
// (WasmBaselineCompile.cpp, WasmIonCompile.cpp, WasmBCFrame.cpp,
// WasmBCMemory.cpp) which are excluded on platforms without a JIT backend
// (JS_WITHOUT_WASM / WASI).

#include "wasm/WasmBaselineCompile.h"
#include "wasm/WasmCompile.h"  // IonDumpContents
#include "wasm/WasmIonCompile.h"

using namespace js;
using namespace js::wasm;

// --- WasmBaselineCompile.cpp stubs ---

bool js::wasm::BaselinePlatformSupport() { return false; }

bool js::wasm::BaselineCompileFunctions(
    const CodeMetadata& codeMeta, const CompilerEnvironment& compilerEnv,
    LifoAlloc& lifo, const FuncCompileInputVector& inputs, CompiledCode* code,
    UniqueChars* error) {
  MOZ_CRASH("JIT-based wasm baseline compiler is not available");
}

BaseLocalIter::BaseLocalIter(const ValTypeVector& locals,
                             const ArgTypeVector& args, bool debugEnabled)
    : locals_(locals),
      args_(args),
      argsIter_(args),
      index_(0),
      frameSize_(0),
      nextFrameSize_(0),
      frameOffset_(INT32_MAX),
      stackResultPointerOffset_(INT32_MAX),
      mirType_(jit::MIRType::Undefined),
      done_(true) {}

void BaseLocalIter::operator++(int) {
  MOZ_CRASH("BaseLocalIter: no wasm compiler available");
}

// settle() and pushLocal() are private helpers called from the constructor
// and operator++.  Since the stub constructor sets done_ = true and operator++
// crashes, they are never reached.
void BaseLocalIter::settle() {
  MOZ_CRASH("BaseLocalIter: no wasm compiler available");
}

int32_t BaseLocalIter::pushLocal(size_t nbytes) {
  MOZ_CRASH("BaseLocalIter: no wasm compiler available");
}

// --- WasmIonCompile.cpp stubs ---

bool js::wasm::IonPlatformSupport() { return false; }

bool js::wasm::IonCompileFunctions(const CodeMetadata& codeMeta,
                                   const CodeTailMetadata* codeTailMeta,
                                   const CompilerEnvironment& compilerEnv,
                                   LifoAlloc& lifo,
                                   const FuncCompileInputVector& inputs,
                                   CompiledCode* code, UniqueChars* error) {
  MOZ_CRASH("JIT-based wasm Ion compiler is not available");
}

bool js::wasm::IonDumpFunction(const CompilerEnvironment& compilerEnv,
                               const CodeMetadata& codeMeta,
                               const FuncCompileInput& func,
                               IonDumpContents contents, GenericPrinter& out,
                               UniqueChars* error) {
  MOZ_CRASH("JIT-based wasm Ion compiler is not available");
}
