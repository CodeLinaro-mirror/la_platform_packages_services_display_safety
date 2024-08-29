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

// TODO(b/334163598): remove the dependency of JNI on libsdv_dt.
#include "SdvGatewayImpl.h"
#include "libsdv_dt.h"

#include <android-base/logging.h>
#include <log/log.h>

#include <jni.h>

#include <algorithm>
#include <map>
#include <memory>
#include <string>

namespace {

using android::sdv::dt::SubscriptionNotificationType;
using android::sdv::gateway::SdvGatewayErrorCode;
using android::sdv::gateway::SdvGatewayImpl;
using android::sdv::gateway::SdvGatewayStatus;

// Global variables
JavaVM* g_vm;
jobject g_obj;
jmethodID g_method_id;

std::unique_ptr<SdvGatewayStatus> createSdvGatewayStatus(bool success, SdvGatewayErrorCode err,
                                                         const char* msg) {
    std::unique_ptr<SdvGatewayStatus> status = std::make_unique<SdvGatewayStatus>();
    if (!status) {
        return status;
    }

    status->set_success(success);
    status->set_error_code(err);
    status->set_error_message(msg);
    return status;
}

jbyteArray serializeStatus(JNIEnv* env, std::unique_ptr<SdvGatewayStatus> status) {
    const int byte_size = status->ByteSizeLong();
    jbyteArray status_array = env->NewByteArray(byte_size);
    void* status_array_ptr = env->GetPrimitiveArrayCritical(status_array, /*isCopy=*/nullptr);
    status->SerializeWithCachedSizesToArray(static_cast<uint8_t*>(status_array_ptr));
    env->ReleasePrimitiveArrayCritical(status_array, status_array_ptr, JNI_ABORT);
    return status_array;
}

jstring nativeGetVersion(JNIEnv* env, jobject /*obj*/) {
    auto version = SdvGatewayImpl::GetInstance()->getVersion();
    return env->NewStringUTF(version.c_str());
}

jbyteArray nativeInitSdvComms(JNIEnv* env, jobject obj, jbyteArray identity_key,
                              jstring package_name, jstring service_name) {
    bool jni_init_success = true;

    const jsize identity_key_size = env->GetArrayLength(identity_key);
    const jbyte* identity_key_data = env->GetByteArrayElements(identity_key, nullptr);
    SdvGatewayImpl::IdentityPublicKey key;
    if (std::copy_n(identity_key_data, identity_key_size, key.begin()) == key.begin()) {
        return serializeStatus(env,
                               createSdvGatewayStatus(false, SdvGatewayErrorCode::JNI,
                                                      "Failed to populate an identity key."));
    }

    const char* package_name_str = env->GetStringUTFChars(package_name, nullptr);
    const char* service_name_str = env->GetStringUTFChars(service_name, nullptr);

    // global variables Setup for Java methods callback
    g_obj = env->NewGlobalRef(obj);

    jclass clazz = env->GetObjectClass(g_obj);
    if (clazz == NULL) {
        return serializeStatus(env,
                               createSdvGatewayStatus(false, SdvGatewayErrorCode::JNI,
                                                      "Failed to find SdvConnectionManager class"));
    }

    g_method_id = env->GetMethodID(clazz, "onEvent", "([BLjava/lang/String;)V");
    if (g_method_id == NULL) {
        return serializeStatus(env,
                               createSdvGatewayStatus(false, SdvGatewayErrorCode::JNI,
                                                      "Unable to get a reference to onEvent() "
                                                      "methode."));
    }

    return serializeStatus(env,
                           SdvGatewayImpl::GetInstance()->initComms(key,
                                                                    std::string(package_name_str),
                                                                    std::string(service_name_str)));
}

jbyteArray nativeConnectToServer(JNIEnv* env, jobject /*obj*/, jstring server_package_name,
                                 jstring server_name, jstring client_name) {
    const char* server_package_name_str = env->GetStringUTFChars(server_package_name, nullptr);
    const char* server_name_str = env->GetStringUTFChars(server_name, nullptr);
    const char* client_name_str = env->GetStringUTFChars(client_name, nullptr);
    if (server_package_name_str == nullptr || server_name_str == nullptr ||
        client_name_str == nullptr) {
        return serializeStatus(env,
                               createSdvGatewayStatus(false, SdvGatewayErrorCode::JNI,
                                                      "Invalid argument."));
    }

    return serializeStatus(env,
                           SdvGatewayImpl::GetInstance()
                                   ->connectToServer(std::string(server_package_name_str),
                                                     std::string(server_name_str),
                                                     std::string(client_name_str)));
}

jbyteArray nativeCreateServer(JNIEnv* env, jobject /*obj*/, jstring server_name, jint port) {
    const char* server_name_str = env->GetStringUTFChars(server_name, nullptr);
    auto status = SdvGatewayImpl::GetInstance()->createServer(std::string(server_name_str),
                                                              static_cast<int>(port));
    return serializeStatus(env, std::move(status));
}

void NotificationHandler(const std::string& topic_name, SubscriptionNotificationType type) {
    if (type == SubscriptionNotificationType::DataAvailable) {
        JNIEnv* env = nullptr;
        int get_env_stat = g_vm->GetEnv((void**)&env, JNI_VERSION_1_6);
        if (get_env_stat == JNI_EDETACHED) {
            if (g_vm->AttachCurrentThread((JNIEnv**)&env, NULL) != 0) {
                ALOGE("Failed to attach");
            }
        } else if (get_env_stat != JNI_OK) {
            ALOGE("GetEnv: status is Not JNI_OK");
        }
        CHECK(env != nullptr) << "JNI Env is null";
        // Call libsdvgateway to parse and get the list of objects
        // TODO(b/335457833): Handle multiple messages when message_count != 1
        int message_count = 0;
        const auto messages =
                SdvGatewayImpl::GetInstance()->readMessagesForTopic(topic_name, &message_count);
        // Call onEventMethod on Java app to handle the parsed data
        const jstring topic_name_for_java = env->NewStringUTF(topic_name.c_str());
        jbyteArray bytes_for_java = env->NewByteArray(messages.size() * message_count);
        env->SetByteArrayRegion(bytes_for_java, 0, messages.size() * message_count,
                                reinterpret_cast<const signed char*>(messages.data()));
        env->CallVoidMethod(g_obj, g_method_id, bytes_for_java, topic_name_for_java);

        if (env->ExceptionCheck()) {
            env->ExceptionDescribe();
        }
        g_vm->DetachCurrentThread();
    }
}

jbyteArray nativeSubscribeToTopic(JNIEnv* env, jobject /*obj*/, jstring topic_name) {
    const char* topic_name_str = env->GetStringUTFChars(topic_name, nullptr);
    auto status = SdvGatewayImpl::GetInstance()->subscribeToTopic(std::string(topic_name_str),
                                                                  NotificationHandler);
    return serializeStatus(env, std::move(status));
}

jbyteArray nativeRegisterTopic(JNIEnv* env, jobject /*obj*/, jstring topic_name, jlong message_size,
                               jlong message_count) {
    const char* topic_name_str = env->GetStringUTFChars(topic_name, nullptr);
    if (message_size < 0 || message_count < 0) {
        jclass exceptionClass = env->FindClass("java/lang/IllegalArgumentException");
        if (exceptionClass != nullptr) {
            env->ThrowNew(exceptionClass, "Message size and count cannot be negative");
        }
    }

    const auto native_message_size = static_cast<size_t>(message_size);
    const auto native_message_count = static_cast<size_t>(message_count);
    auto status =
            SdvGatewayImpl::GetInstance()->registerTopic(std::string(topic_name_str),
                                                         native_message_size, native_message_count);
    return serializeStatus(env, std::move(status));
}

jbyteArray nativePublishToTopic(JNIEnv* env, jobject /*obj*/, jstring topic_name,
                                jbyteArray message) {
    const char* topic_name_str = env->GetStringUTFChars(topic_name, nullptr);
    const jsize len = env->GetArrayLength(message);
    jbyte* native_message = env->GetByteArrayElements(message, nullptr);
    std::vector<uint8_t> byte_vector(len);
    std::copy_n(native_message, len, byte_vector.begin());
    env->ReleaseByteArrayElements(message, native_message, JNI_ABORT);
    auto status =
            SdvGatewayImpl::GetInstance()->publishToTopic(std::string(topic_name_str), byte_vector);
    return serializeStatus(env, std::move(status));
}

static JNINativeMethod gMethods[] = {
        {"nativeGetVersion", "()Ljava/lang/String;", reinterpret_cast<void*>(nativeGetVersion)},
        {"nativeInitSdvComms", "([BLjava/lang/String;Ljava/lang/String;)[B",
         reinterpret_cast<void*>(nativeInitSdvComms)},
        {"nativeConnectToServer", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)[B",
         reinterpret_cast<void*>(nativeConnectToServer)},
        {"nativeCreateServer", "(Ljava/lang/String;I)[B",
         reinterpret_cast<void*>(nativeCreateServer)},
        {"nativeSubscribeToTopic", "(Ljava/lang/String;)[B",
         reinterpret_cast<void*>(nativeSubscribeToTopic)},
        {"nativeRegisterTopic", "(Ljava/lang/String;JJ)[B",
         reinterpret_cast<void*>(nativeRegisterTopic)},
        {"nativePublishToTopic", "(Ljava/lang/String;[B)[B",
         reinterpret_cast<void*>(nativePublishToTopic)},
};

}  // namespace

jint registerSdvConnectionManager(JavaVM* vm, JNIEnv* env) {
    g_vm = vm;
    const jclass clazz =
            env->FindClass("com/android/car/displaysafety/camera/SdvConnectionManagerImpl");
    if (clazz == nullptr) {
        ALOGE("Could not find class to register native functions");
        return JNI_ERR;
    }
    return env->RegisterNatives(clazz, gMethods, sizeof(gMethods) / sizeof(JNINativeMethod));
}

