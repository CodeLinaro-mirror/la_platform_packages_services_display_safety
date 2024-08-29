/*
 * Copyright 2024 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <android-base/logging.h>
#include <log/log.h>

#include <jni.h>

#ifdef __ENABLED__
extern jint registerSdvConnectionManager(JavaVM* vm, JNIEnv* env);
#else
namespace {

jint registerSdvConnectionManager([[maybe_unused]] JavaVM* vm, [[maybe_unused]] JNIEnv* env) {
    // No-op method.
    return JNI_OK;
}

} // namespace
#endif

JNIEXPORT jint JNI_OnLoad(JavaVM* vm, void* /*reserved*/) {
    JNIEnv* env = nullptr;
    if (vm->GetEnv((void**)&env, JNI_VERSION_1_6) != JNI_OK) return JNI_ERR;
    if (registerSdvConnectionManager(vm, env) != JNI_OK) {
        ALOGE("Failed to register SDV connection manager.");
        return JNI_ERR;
    }
    return JNI_VERSION_1_6;
}
