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
