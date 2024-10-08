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

# This contains the setup required to run Harry on SDV media.
# In this configuration Harry is the only app drawing on the display.
# All binaries and packages are packaged as executables installed on the
# system, without using APEXes.

# Check if the home dir was defined.
ifeq ($(DISPLAY_SAFETY_HOME),)
$(error DISPLAY_SAFETY_HOME must be defined.)
endif #DISPLAY_SAFETY_HOME

# Add product specific sepolicies
BOARD_VENDOR_SEPOLICY_DIRS += \
    $(DISPLAY_SAFETY_HOME)/product/harry_standalone/sepolicy \

# Add Harry and SDV libraries/services
PRODUCT_PACKAGES += \
    harry \
    har_user_preferences_service \
    har_preferences_service \
    vehicledata_publisher_service \
    fake_vehicledata_publisher_service \
    har_sdv_service \

PRODUCT_COPY_FILES += \
    vendor/google/display_safety/service/product/harry_standalone/harry_init.rc:/vendor/etc/init/harry_init.rc \
    vendor/google/display_safety/service/product/harry_standalone/init_data_folder.sh:/vendor/etc/harry/com.google.display_safety.har.init_data_folder.sh

# Resources
PRODUCT_PACKAGES += \
        harry_res_config \
        harry_res_documents \
        harry_res_fonts \
        harry_res_audio_config \
        harry_res_chimes_config \
        harry_res_audio_chimes \
        harry_res_camera_config \
        harry_res_emulated_camera_config \
        harry_res_camera_emulation_source \
        harry_res_locales \
        har_preferences_admin \
