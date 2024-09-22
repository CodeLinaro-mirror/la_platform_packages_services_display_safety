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

#include <aidl/google/sdv/gateway/PublicKey.h>
#include <aidl/google/sdv/gateway/RawMessage.h>
#include <aidl/google/sdv/gateway/ResultStatus.h>
#include <aidl/google/sdv/gateway/SdvGatewayStatusCode.h>
#include <android-base/logging.h>
#include <log/log.h>

#include <jni.h>

#include <algorithm>
#include <map>
#include <memory>
#include <string>

namespace {

using aidl::google::sdv::gateway::PublicKey;
using aidl::google::sdv::gateway::RawMessage;
using aidl::google::sdv::gateway::ResultStatus;
using aidl::google::sdv::gateway::SdvGatewayStatusCode;
using android::sdv::dt::SubscriptionNotificationType;
using android::sdv::gateway::SdvGatewayImpl;

// Global variables
JavaVM* g_vm;
jobject g_obj;
jmethodID g_method_id;

jobject createResultStatus(JNIEnv* env, const ResultStatus& status) {
    const jclass clazz = env->FindClass("google/sdv/gateway/ResultStatus");
    if (clazz == NULL) {
        LOG(ERROR) << "Could not find ResultStatus class.";
        return NULL;
    }

    jmethodID methodId = env->GetMethodID(clazz, "<init>", "()V");
    if (methodId == NULL) {
        LOG(ERROR) << "Could not find the constructor of ResultStatus class.";
        return NULL;
    }

    jobject classObj = env->NewObject(clazz, methodId);
    if (classObj == NULL) {
        LOG(ERROR) << "Failed to instantiate ResultStatus class.";
        return NULL;
    }

    jfieldID fieldId = env->GetFieldID(clazz, "statusCode", "I");
    if (fieldId == NULL) {
        LOG(ERROR) << "Could not get statusCode field from ResultStatus instance.";
        return NULL;
    }

    env->SetIntField(classObj, fieldId, static_cast<int>(status.statusCode));

    if (status.errorMessage.has_value()) {
        jfieldID errorId = env->GetFieldID(clazz, "errorMessage", "Ljava/lang/String;");
        if (errorId == NULL) {
            LOG(ERROR) << "Could not get errorMessage field.";
            return NULL;
        }
        env->SetObjectField(classObj, errorId,
                            env->NewStringUTF(status.errorMessage.value().data()));
    }

    if (status.returnValue.has_value()) {
        jfieldID valueId = env->GetFieldID(clazz, "returnValue", "Ljava/lang/String;");
        if (valueId == NULL) {
            LOG(ERROR) << "Could not get returnValue field.";
            return NULL;
        }
        env->SetObjectField(classObj, valueId,
                            env->NewStringUTF(status.returnValue.value().data()));
    }

    return classObj;
}

jobject createRawMessageArrayList(JNIEnv* env, const std::vector<RawMessage>& msgs) {
    jclass clazz = env->FindClass("java/util/ArrayList");
    if (clazz == NULL) {
        LOG(ERROR) << "Could not find java.util.ArrayList.";
        return NULL;
    }

    jmethodID methodId = env->GetMethodID(clazz, "<init>", "(I)V");
    if (methodId == NULL) {
        LOG(ERROR) << "Failed to get the constructor of ArrayList class.";
        return NULL;
    }

    jobject listObj = env->NewObject(clazz, methodId, msgs.size());
    if (listObj == NULL) {
        LOG(ERROR) << "Failed to instantiate ArrayList class.";
        return NULL;
    }

    jmethodID addMethodId = env->GetMethodID(clazz, "add", "(Ljava/lang/Object;)Z");
    if (addMethodId == NULL) {
        LOG(ERROR) << "Failed to get ArrayList.add method.";
        return NULL;
    }

    clazz = env->FindClass("google/sdv/gateway/RawMessage");
    if (clazz == NULL) {
        LOG(ERROR) << "Could not find google.sdv.gateway.RawMessage class.";
        return NULL;
    }

    methodId = env->GetMethodID(clazz, "<init>", "()V");
    if (methodId == NULL) {
        LOG(ERROR) << "Failed to get the constructor of RawMessage class.";
        return NULL;
    }

    jfieldID fieldId = env->GetFieldID(clazz, "data", "[B");
    if (fieldId == NULL) {
        LOG(ERROR) << "Failed to get data field of RawMessage instance.";
        return NULL;
    }

    for (const auto& msg : msgs) {
        jobject obj = env->NewObject(clazz, methodId);
        if (obj == NULL) {
            LOG(ERROR) << "Failed to instantiate RawMessage class.";
            continue;
        }

        jbyteArray byteArrayObj = env->NewByteArray(msg.data.size());
        if (byteArrayObj == NULL) {
            LOG(ERROR) << "Failed to instantiate byte[] object.";
            continue;
        }

        env->SetByteArrayRegion(byteArrayObj, 0, msg.data.size(),
                                reinterpret_cast<const signed char*>(msg.data.data()));
        env->SetObjectField(obj, fieldId, byteArrayObj);
        env->CallBooleanMethod(listObj, addMethodId, obj);
    }

    return listObj;
}

jobject createResultStatusFromValues(JNIEnv* env, SdvGatewayStatusCode code, const char* msg) {
    return createResultStatus(env,
                              ResultStatus{
                                      .statusCode = code,
                                      .errorMessage = std::make_optional(msg),
                              });
}

jstring nativeGetVersion(JNIEnv* env, jobject /*obj*/) {
    auto version = SdvGatewayImpl::GetInstance()->getVersion();
    return env->NewStringUTF(version.c_str());
}

jobject nativeInitSdvComms(JNIEnv* env, jobject obj, jbyteArray identity_key, jstring package_name,
                           jstring service_name) {
    bool jni_init_success = true;

    const jsize identity_key_size = env->GetArrayLength(identity_key);
    const jbyte* identity_key_data = env->GetByteArrayElements(identity_key, nullptr);

    PublicKey key;
    if (std::copy_n(identity_key_data, identity_key_size, key.keyValue.begin()) ==
        key.keyValue.begin()) {
        LOG(ERROR) << "Failed to populate an identity key.";
        return createResultStatusFromValues(env, SdvGatewayStatusCode::JNI, "JNI init failure");
    }

    const char* package_name_str = env->GetStringUTFChars(package_name, nullptr);
    const char* service_name_str = env->GetStringUTFChars(service_name, nullptr);

    // global variables Setup for Java methods callback
    g_obj = env->NewGlobalRef(obj);

    jclass clazz = env->GetObjectClass(g_obj);
    if (clazz == NULL) {
        LOG(ERROR) << "Failed to find SdvConnectionManager class";
        return createResultStatusFromValues(env, SdvGatewayStatusCode::JNI, "JNI init failure");
    }

    g_method_id =
            env->GetMethodID(clazz, "onMessagesAvailable", "(Ljava/lang/String;Ljava/util/List;)V");
    if (g_method_id == NULL) {
        LOG(ERROR) << "Unable to get a reference to onMessagesAvailable method.";
        return createResultStatusFromValues(env, SdvGatewayStatusCode::JNI, "JNI init failure");
    }

    return createResultStatus(env,
                              SdvGatewayImpl::GetInstance()
                                      ->initComms(key, std::string(package_name_str),
                                                  std::string(service_name_str)));
}

jobject nativeConnectToServer(JNIEnv* env, jobject /*obj*/, jstring server_package_name,
                              jstring server_name, jstring client_name) {
    const char* server_package_name_str = env->GetStringUTFChars(server_package_name, nullptr);
    const char* server_name_str = env->GetStringUTFChars(server_name, nullptr);
    const char* client_name_str = env->GetStringUTFChars(client_name, nullptr);
    if (server_package_name_str == nullptr || server_name_str == nullptr ||
        client_name_str == nullptr) {
        return createResultStatusFromValues(env, SdvGatewayStatusCode::JNI, "Invalid argument.");
    }

    return createResultStatus(env,
                              SdvGatewayImpl::GetInstance()
                                      ->connectToServer(std::string(server_package_name_str),
                                                        std::string(server_name_str),
                                                        std::string(client_name_str)));
}

jobject nativeCreateServer(JNIEnv* env, jobject /*obj*/, jstring server_name, jint port) {
    const char* server_name_str = env->GetStringUTFChars(server_name, nullptr);
    return createResultStatus(env,
                              SdvGatewayImpl::GetInstance()->createServer(std::string(
                                                                                  server_name_str),
                                                                          static_cast<int>(port)));
}

void NotificationHandler(const std::string& topicName, const std::vector<RawMessage>& rawMessages) {
    CHECK(!rawMessages.empty()) << "Notification with empty messages vector";
    JNIEnv* env = nullptr;
    int get_env_stat = g_vm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (get_env_stat == JNI_EDETACHED) {
        if (g_vm->AttachCurrentThread((JNIEnv**)&env, NULL) != 0) {
            LOG(ERROR) << "Failed to attach.";
            return;
        }
    } else if (get_env_stat != JNI_OK) {
        LOG(ERROR) << "GetEnv: status is Not JNI_OK";
        return;
    }
    CHECK(env != nullptr) << "JNI Env is null";

    // Call onEventMethod on Java app to handle the parsed data
    const jstring jTopicName = env->NewStringUTF(topicName.c_str());
    jobject jRawMessages = createRawMessageArrayList(env, rawMessages);
    env->CallVoidMethod(g_obj, g_method_id, jTopicName, jRawMessages);

    if (env->ExceptionCheck()) {
        env->ExceptionDescribe();
    }
    g_vm->DetachCurrentThread();
}

jobject nativeSubscribeToTopic(JNIEnv* env, jobject /*obj*/, jstring topic_name) {
    jclass clazz = env->FindClass("google/sdv/gateway/RawMessage");
    if (clazz == NULL) {
        return createResultStatusFromValues(env, SdvGatewayStatusCode::JNI,
                                            "Could not find RawMessage class.");
    }

    const char* topic_name_str = env->GetStringUTFChars(topic_name, nullptr);
    return createResultStatus(env,
                              SdvGatewayImpl::GetInstance()
                                      ->subscribeToTopic(SdvGatewayImpl::SessionId{},
                                                         std::string(topic_name_str),
                                                         NotificationHandler));
}

jobject nativeRegisterTopic(JNIEnv* env, jobject /*obj*/, jstring topic_name, jlong message_size,
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
    return createResultStatus(env,
                              SdvGatewayImpl::GetInstance()->registerTopic(std::string(
                                                                                   topic_name_str),
                                                                           native_message_size,
                                                                           native_message_count));
}

jobject nativePublishToTopic(JNIEnv* env, jobject /*obj*/, jstring topic_name, jbyteArray message) {
    const char* topic_name_str = env->GetStringUTFChars(topic_name, nullptr);
    const jsize len = env->GetArrayLength(message);
    jbyte* native_message = env->GetByteArrayElements(message, nullptr);
    std::vector<uint8_t> byte_vector(len);
    std::copy_n(native_message, len, byte_vector.begin());
    env->ReleaseByteArrayElements(message, native_message, JNI_ABORT);
    return createResultStatus(env,
                              SdvGatewayImpl::GetInstance()->publishToTopic(std::string(
                                                                                    topic_name_str),
                                                                            byte_vector));
}

JNINativeMethod gMethods[] = {
        {"nativeGetVersion", "()Ljava/lang/String;", reinterpret_cast<void*>(nativeGetVersion)},
        {"nativeInitSdvComms",
         "([BLjava/lang/String;Ljava/lang/String;)Lgoogle/sdv/gateway/ResultStatus;",
         reinterpret_cast<void*>(nativeInitSdvComms)},
        {"nativeConnectToServer",
         "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Lgoogle/sdv/gateway/"
         "ResultStatus;",
         reinterpret_cast<void*>(nativeConnectToServer)},
        {"nativeCreateServer", "(Ljava/lang/String;I)Lgoogle/sdv/gateway/ResultStatus;",
         reinterpret_cast<void*>(nativeCreateServer)},
        {"nativeSubscribeToTopic", "(Ljava/lang/String;)Lgoogle/sdv/gateway/ResultStatus;",
         reinterpret_cast<void*>(nativeSubscribeToTopic)},
        {"nativeRegisterTopic", "(Ljava/lang/String;JJ)Lgoogle/sdv/gateway/ResultStatus;",
         reinterpret_cast<void*>(nativeRegisterTopic)},
        {"nativePublishToTopic", "(Ljava/lang/String;[B)Lgoogle/sdv/gateway/ResultStatus;",
         reinterpret_cast<void*>(nativePublishToTopic)},
};

}  // namespace

jint registerSdvConnectionManager(JavaVM* vm, JNIEnv* env) {
    g_vm = vm;
    const jclass clazz =
            env->FindClass("com/android/car/displaysafety/camera/SdvConnectionManagerImpl");
    if (clazz == nullptr) {
        LOG(ERROR) << "Could not find class to register native functions";
        return JNI_ERR;
    }

    return env->RegisterNatives(clazz, gMethods, sizeof(gMethods) / sizeof(JNINativeMethod));
}
