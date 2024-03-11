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

#include "HarGraphics.h"
#include "GLDriver.h"

android::GLDriver gl_driver;

int har_init_gl(int display_id) {
    return gl_driver.init_gl(display_id) ? 1 : 0;
}

__harMustCastToProperFunctionPointerType har_get_process_address(const char *procname) {
    return gl_driver.get_process_address(procname);
}

int har_swap_buffers() {
    return gl_driver.swap_buffers() ? 1 : 0;
}

int har_make_current() {
    return gl_driver.make_current() ? 1 : 0;
}