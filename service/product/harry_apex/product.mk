#
# Copyright (C) 2024 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# This will package the HAR reference implementation as an APEX.
# The used SDV services are also packaged as part of the HAR APEX or
# in separate SDV Service Bundle(s).

# Enable APEX updates for this product.
$(call inherit-product, $(SRC_TARGET_DIR)/product/updatable_apex.mk)

# Add product specific sepolicies
BOARD_VENDOR_SEPOLICY_DIRS += \
    vendor/google/display_safety/service/product/harry_apex/sepolicy \

# Allow HAR SDV Service Bundles to use sockets.
SYSTEM_EXT_PRIVATE_SEPOLICY_DIRS += \
    vendor/google/display_safety/service/product/har_sdv_service_bundle_apex/lifecycle/sepolicy \

# Set this to true for testing with the stub app.
HAR_USE_STUB_APP ?= false

ifeq ($(HAR_USE_STUB_APP), true)
PRODUCT_PACKAGES += \
    com.google.display_safety.har.stub
else
# Add Harry APEX bundle with all required packages.
PRODUCT_PACKAGES += \
    com.google.display_safety.har
endif  # HAR_USE_STUB_APP

# Add SDV Services
PRODUCT_PACKAGES += \
    com.sdv.google.display_safety.services_bundle.apex \
