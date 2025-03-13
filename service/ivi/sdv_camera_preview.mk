# Copyright 2024 Google LLC
#
# sdv_camera_preview
# This file adds SdvCameraPreviewApp and related configs

PRODUCT_PACKAGES += \
    SdvCameraPreviewApp \
    CarServiceSdvCameraPreviewAppRRO \

# TODO(b/321998205): Remove this additional package.
PRODUCT_PACKAGES += SdvCameraPreviewAppPrebuilt

SOONG_CONFIG_NAMESPACES += sdvcamerapreviewapp
SOONG_CONFIG_sdvcamerapreviewapp += enabled
SOONG_CONFIG_sdvcamerapreviewapp_enabled := true

# TODO(b/380467672): Temporarily, we are using a mock EVS HAL and its emulated cameras
#                    on CF-based targets for display-safety.
ENABLE_MOCK_EVSHAL ?= true
ENABLE_EVS_SAMPLE ?= false
