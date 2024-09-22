/*
 * Copyright (C) 2024 The Android Open Source Project
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

package com.android.car.displaysafety.camera;

import android.util.Log;

import androidx.annotation.GuardedBy;
import androidx.annotation.NonNull;

import com.google.protobuf.InvalidProtocolBufferException;
import google.sdv.gateway.ISdvGatewaySession;
import google.sdv.gateway.PublicKey;
import google.sdv.gateway.RawMessage;
import google.sdv.gateway.ResultStatus;
import google.sdv.gateway.SdvGatewayStatusCode;

import io.grpc.ChannelCredentials;
import io.grpc.Grpc;
import io.grpc.InsecureChannelCredentials;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.Status;
import io.grpc.StatusException;
import io.grpc.TlsChannelCredentials;

import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;

public final class SdvConnectionManagerImpl implements SdvConnectionManager, TopicDataListener {
    private static final String TAG = SdvConnectionManagerImpl.class.getSimpleName();

    private final static class ChannelInfo {
        public String host;
        public int port;
    }

    private final HashMap<String, List<DataTunnelCallback>> mDataTunnelCallbacks = new HashMap<>();
    private final Object mDataTunnelListenerLock = new Object();

    private static void loadLibrary(String name) {
        System.loadLibrary(name);
    }

    // TODO(b/341370027): App should retry DT subscribes periodically.
    private boolean subscribeToTopic(String topicName) {
        var status = nativeSubscribeToTopic(topicName);
        if (status.statusCode == SdvGatewayStatusCode.OK) {
            return true;
        }

        Log.e(TAG, "Subscribe to topic failed with error " + status.errorMessage);
        return false;
    }

    private SdvConnectionManagerImpl() {}

    private boolean initSdvComms(byte[] identityKey, String packageName, String appName) {
        ResultStatus status = nativeInitSdvComms(identityKey, packageName, appName);
        if (status.statusCode != SdvGatewayStatusCode.OK) {
            Log.e(TAG, "SDV Gateway initSdvComms failed with error "
              + status.errorMessage
              + ", cannot start communication with SDV");
            return false;
        }

        Log.i(TAG, "Successfully initialized SDV comm for " + packageName + "/" + appName);
        return true;
    }

    private ManagedChannel obtainSecureManagedChannelInternal(String connectionString)
            throws IOException {

        // TODO: implement this method with proper certificate.
        return null;
    }

    private ManagedChannel obtainInsecureManagedChannelInternal(String connectionString)
            throws IOException {
        ChannelInfo channelInfo = parseConnectionString(connectionString);
        ManagedChannel channel = Grpc.newChannelBuilderForAddress(
                channelInfo.host, channelInfo.port, InsecureChannelCredentials.create()).build();

        return channel;
    }

    private ChannelInfo parseConnectionString(String connectionString) {
        String[] connectionStringTokens = connectionString.split(":");
        ChannelInfo channelInfo = new ChannelInfo();
        channelInfo.host = connectionStringTokens[0];
        channelInfo.port = Integer.parseInt(connectionStringTokens[1]);

        return channelInfo;
    }

    private native String nativeGetVersion();

    private native ResultStatus nativeInitSdvComms(
            byte[] identityKey, String packageName, String serviceName);

    private native ResultStatus nativeConnectToServer(
            String serverPackageName, String serverName, String clientName);

    private native ResultStatus nativeCreateServer(String serverName, int port);

    private native ResultStatus nativeSubscribeToTopic(String topicname);

    private native ResultStatus nativeRegisterTopic(String topicname, long messageSize,
            long messageCount);

    private native ResultStatus nativePublishToTopic(String topicname, byte[] message);

    static {
        System.loadLibrary("harsdvgateway_jni");
    }

    public static SdvConnectionManager Create(byte[] identityKey, String packageName,
            String appName, String servicePackageName, String serviceName) {

        SdvConnectionManagerImpl mgr = new SdvConnectionManagerImpl();
        if (mgr == null) {
            Log.e(TAG, "Failed to instantiate SdvConnectionManager class.");
            return null;
        }

        if (!mgr.initSdvComms(identityKey, packageName, appName)) {
            Log.e(TAG, "Failed to initialize SDV Comm.");
            return null;
        }

        // Attempt to connect to the target service.
        ResultStatus status = mgr.nativeConnectToServer(servicePackageName, serviceName, appName);
        if (status.statusCode != SdvGatewayStatusCode.OK) {
            Log.e(TAG, "Failed to connect the service: " + servicePackageName +
                    "/" + serviceName + " Error: " + status.errorMessage);
            return null;
        }

        Log.i(TAG, "Connected to the service: " + status.returnValue);
        return (SdvConnectionManager) mgr;
    }

    @Override
    public boolean createServer(String serverName, int port) {
        ResultStatus status = nativeCreateServer(serverName, port);
        if (status.statusCode == SdvGatewayStatusCode.OK) {
            return true;
        }

        Log.e(TAG, "Creating rpc server failed with error " + status.errorMessage);
        return false;
    }

    @Override
    public boolean registerTopic(String topicName, long messageSize, long messageCount) {
        ResultStatus status = nativeRegisterTopic(topicName, messageSize, messageCount);
        if (status.statusCode == SdvGatewayStatusCode.OK) {
            return true;
        }

        Log.e(TAG, "Registering DT topic failed with error " + status.errorMessage);
        return false;
    }

    @Override
    public boolean publishToTopic(String topicName, byte[] message) {
        ResultStatus status = nativePublishToTopic(topicName, message);
        if (status.statusCode == SdvGatewayStatusCode.OK) {
            return true;
        }

        Log.e(TAG, "Publish to topic failed with error " + status.errorMessage);
        return false;
    }

    @Override
    public boolean registerDataTunnelCallback(@NonNull DataTunnelCallback cb, String topicName) {
        if (cb == null) {
            Log.e(TAG, "registerDataTunnelCallback(): null listener");
            return false;
        }

        boolean isNewTopicName;
        synchronized (mDataTunnelListenerLock) {
            List<DataTunnelCallback> subsCallbacks = mDataTunnelCallbacks.get(topicName);
            isNewTopicName = subsCallbacks == null;
            if (isNewTopicName) {
                Log.i(TAG, "Subscribing to a new topic: " + topicName);
                subsCallbacks = new ArrayList();
            }

            subsCallbacks.add(cb);
            mDataTunnelCallbacks.put(topicName, subsCallbacks);

            if (isNewTopicName) {
                subscribeToTopic(topicName);
            }
        }

        return true;
    }

    @Override
    public void onMessagesAvailable(String topicName, List<RawMessage> rawMessages) {
        Log.d(TAG, "onMessagesAvailable data tunnel callback for topic: " + topicName);
        synchronized (mDataTunnelListenerLock) {
            List<DataTunnelCallback> subsCallback = mDataTunnelCallbacks.get(topicName);
            if (subsCallback == null) {
                Log.w(TAG, "Received an event for an unknown topic, " + topicName);
                return;
            }

            // Forwarding an event to subscribers.
            for (DataTunnelCallback callback : subsCallback) {
                if (callback == null) {
                    // Ignore invalid callback objects.
                    continue;
                }

                for (RawMessage msg : rawMessages) {
                    callback.onEvent(msg.data);
                }
            }
        }
    }

    @Override
    public String getVersionString() {
        return nativeGetVersion();
    }

    @Override
    public ManagedChannel obtainSecureManagedChannel(
            String serverPackageName, String serverName, String clientName)
            throws IOException, StatusException {
        ResultStatus status = nativeConnectToServer(serverPackageName, serverName, clientName);
        if (status.statusCode != SdvGatewayStatusCode.OK) {
            throw new StatusException(Status.NOT_FOUND.withDescription(serverName
                      + " is not available, error: "
                      + status.errorMessage
                      + ", cannot create channel"));
        }

        String connectionString = status.returnValue;
        Log.d(TAG, "Obtained connection string for " + serverName + " Server: " + connectionString);
        return obtainSecureManagedChannelInternal(connectionString);
    }

    @Override
    public ManagedChannel obtainInsecureManagedChannel(
            String serverPackageName, String serverName, String clientName)
            throws IOException, StatusException {
        ResultStatus status = nativeConnectToServer(serverPackageName, serverName, clientName);
        if (status.statusCode != SdvGatewayStatusCode.OK) {
            throw new StatusException(Status.NOT_FOUND.withDescription(serverName
                      + " is not available, error: "
                      + status.errorMessage
                      + ", cannot create channel"));
        }

        return obtainInsecureManagedChannelInternal(status.returnValue);
    }
}
