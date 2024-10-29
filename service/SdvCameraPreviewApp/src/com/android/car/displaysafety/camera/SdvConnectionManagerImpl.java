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
import google.sdv.gateway.client.SdvGatewayClient;
import google.sdv.gateway.client.SdvGatewayRuntimeException;
import google.sdv.gateway.client.SdvGatewayClient;

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

    private SdvConnectionManagerImpl() {}

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

    private native void nativeInitSdvComms(
            byte[] identityKey, String packageName, String serviceName);

    private native String nativeConnectToServer(
            String serverPackageName, String serverName, String clientName);

    private native void nativeCreateServer(String serverName, int port);

    private native void nativeSubscribeToTopic(String topicname);

    private native void nativeRegisterTopic(String topicname, long messageSize,
            long messageCount);

    private native void nativePublishToTopic(String topicname, byte[] message);

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

        mgr.nativeInitSdvComms(identityKey, packageName, appName);

        // Attempt to connect to the target service.
        String connectionString = mgr.nativeConnectToServer(servicePackageName, serviceName, appName);
        Log.i(TAG, "Connected to the service: " + connectionString);
        return (SdvConnectionManager) mgr;
    }

    @Override
    public void createServer(String serverName, int port) {
        nativeCreateServer(serverName, port);
    }

    @Override
    public void registerTopic(String topicName, long messageSize, long messageCount) {
        nativeRegisterTopic(topicName, messageSize, messageCount);
    }

    @Override
    public void publishToTopic(String topicName, byte[] message) {
        nativePublishToTopic(topicName, message);
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
                nativeSubscribeToTopic(topicName);
            }
        }

        return true;
    }

    @Override
    public void onMessagesAvailable(String topicName, List<byte[]> rawMessages) {
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


                for (byte[] data : rawMessages) {
                    callback.onEvent(data);
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

        String connectionString = nativeConnectToServer(serverPackageName, serverName, clientName);
        Log.d(TAG, "Obtained connection string for " + serverName + " Server: " + connectionString);
        return obtainSecureManagedChannelInternal(connectionString);
    }

    @Override
    public ManagedChannel obtainInsecureManagedChannel(
            String serverPackageName, String serverName, String clientName)
            throws IOException, StatusException {
        String connectionString = nativeConnectToServer(serverPackageName, serverName, clientName);
        Log.d(TAG, "Obtained connection string for " + serverName + " Server: " + connectionString);
        return obtainInsecureManagedChannelInternal(connectionString);
    }
}
