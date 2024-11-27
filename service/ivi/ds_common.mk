# Copyright 2024 Google LLC
#
# ds_common
# Adds Apps, RROs and Soong configs required for Display Safety

# Set cluster display settings
PRODUCT_COPY_FILES += \
    vendor/google/display_safety/service/ivi/files/vendor/etc/display_settings.xml:$(TARGET_COPY_OUT_VENDOR)/etc/display_settings.xml

# TODO: b/380354610
# Set default RRO config
#PRODUCT_COPY_FILES += \
#    vendor/google/display_safety/service/ivi/files/product/overlay/config/config.xml:$(TARGET_COPY_OUT_PRODUCT)/overlay/config/config.xml

# Include DriverUI app and enable it using RROs
$(call inherit-product, packages/services/Car/car_product/driverui/driverui_app_and_rros.mk)

# Include dynamic RROs to set HAR client config for DriverUI
$(call inherit-product, vendor/google/display_safety/service/ivi/driverui_har_client_rros.mk)

# Include SdvCameraPreviewApp, related RROs and configs
$(call inherit-product, vendor/google/display_safety/service/ivi/sdv_camera_preview.mk)

PRODUCT_PACKAGES += \
    AutoVoipTestApp
