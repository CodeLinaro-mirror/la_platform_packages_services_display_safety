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

import androidx.annotation.NonNull;

import io.grpc.ChannelCredentials;
import io.grpc.ManagedChannel;
import io.grpc.StatusException;

import java.io.IOException;

/**
 * Abstracts SDV connection manager.
 */
public interface SdvConnectionManager {

    public interface DataTunnelCallback {
        public void onEvent(byte[] content);
    }

    public boolean createServer(String serverName, int port);

    public boolean registerTopic(String topicName, long messageSize, long messageCount);

    public boolean publishToTopic(String topicName, byte[] message);

    public boolean registerDataTunnelCallback(@NonNull DataTunnelCallback cb, String topicName);

    public String getVersionString();

    public ManagedChannel obtainSecureManagedChannel(
            String serverPackageName, String serverName, String clientName)
            throws IOException, StatusException;

    public ManagedChannel obtainInsecureManagedChannel(
            String serverPackageName, String serverName, String clientName)
            throws IOException, StatusException;
}
