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

import android.os.RemoteException;
import android.util.Log;

import androidx.annotation.GuardedBy;
import androidx.annotation.NonNull;

import google.sdv.gateway.client.SdvGatewayClient;
import google.sdv.gateway.client.SdvGatewayRuntimeException;

import io.grpc.Grpc;
import io.grpc.ManagedChannel;
import io.grpc.Status;
import io.grpc.StatusException;

import java.io.IOException;
import java.util.Optional;

public final class SdvConnectionManagerImpl implements SdvConnectionManager {
    private static final String TAG = SdvConnectionManagerImpl.class.getSimpleName();
    private static final String GATEWAY_VERSION_UNKNOWN = "Unknown";

    private final SdvGatewayClient mClient = new SdvGatewayClient();

    private SdvConnectionManagerImpl() {}

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

        Log.i(TAG, "SdvConnectionManager instance is successfully created.");
        return (SdvConnectionManager) mgr;
    }

    @Override
    public boolean initSdvComms(byte[] identityKey, String packageName, String appName) {
        try {
            mClient.initComms(identityKey, packageName, appName);
            return true;
        } catch (RemoteException e) {
            Log.e(TAG, "Failed to initialize SDV Comms due to binder transaction failures.");
        } catch (SdvGatewayRuntimeException e) {
            Log.e(TAG, "Failed to initialize SDV Comms, error = " + e.getSdvGatewayStatusCode() +
                    ", msg = " + e.getMessage());
        }

        return false;
    }

    @Override
    public String getVersionString() {
        try {
            return mClient.getVersion();
        } catch (RemoteException e) {
            Log.e(TAG, "Failed to obtain a version information.");
        }

        return GATEWAY_VERSION_UNKNOWN;
    }

    @Override
    public ManagedChannel obtainManagedChannel(
            String sdvName, String packageName, String bundleName, String unitName)
            throws StatusException {
        try {
            return mClient.connectToRpcServerByName(sdvName, packageName, bundleName, unitName);
        } catch (SdvGatewayRuntimeException e) {
            Log.e(TAG, packageName + "." + bundleName + "." + unitName +
                " is not available, error: " + e.getSdvGatewayStatusCode() + ", msg: " +
                        e.getMessage());
        } catch (IOException | RemoteException e) {
            Log.e(TAG, "Failed to find " + packageName + "." + bundleName + "." + unitName +
                    "due to " + e);
        }

        return null;
    }
}
