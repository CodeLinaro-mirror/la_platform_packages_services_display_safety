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

#define LOG_NDEBUG 1
#define LOG_TAG "HarrySys"
#define ATRACE_TAG ATRACE_TAG_GRAPHICS

#include <vector>

#include <stdint.h>
#include <inttypes.h>
#include <sys/inotify.h>
#include <sys/poll.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <math.h>
#include <fcntl.h>
#include <utils/misc.h>
#include <utils/Trace.h>
#include <signal.h>
#include <time.h>

#include <cutils/atomic.h>
#include <cutils/properties.h>

#include <android/imagedecoder.h>
#include <androidfw/AssetManager.h>
#include <binder/IPCThreadState.h>
#include <utils/Errors.h>
#include <utils/Log.h>
#include <utils/SystemClock.h>

#include <android-base/properties.h>

#include <ui/DisplayMode.h>
#include <ui/DisplayState.h>
#include <ui/PixelFormat.h>
#include <ui/Rect.h>
#include <ui/Region.h>

#include <gui/ISurfaceComposer.h>
#include <gui/DisplayEventReceiver.h>
#include <gui/Surface.h>
#include <gui/SurfaceComposerClient.h>
#include <GLES2/gl2.h>
#include <GLES2/gl2ext.h>
#include <EGL/eglext.h>

#include "GLDriver.h"

#define ANIM_PATH_MAX 255
#define STR(x)   #x
#define STRTO(x) STR(x)

namespace android {

using ui::DisplayMode;

static const char DISPLAYS_PROP_NAME[] = "ro.emulator.car.har.displays";

// ---------------------------------------------------------------------------

// TODO(323902452): destroy/drop context is not implemented.
GLDriver::GLDriver() {
    ATRACE_CALL();
    mSession = new SurfaceComposerClient();

    std::string powerCtl = android::base::GetProperty("sys.powerctl", "");
    mShuttingDown = !powerCtl.empty();
    ALOGD("%sGLDriverStartTiming start time: %" PRId64 "ms", mShuttingDown ? "Shutdown" : "Boot",
            elapsedRealtime());
}

GLDriver::~GLDriver() {
    ATRACE_CALL();

    ALOGD("%sGLDriverStopTiming start time: %" PRId64 "ms", mShuttingDown ? "Shutdown" : "Boot",
            elapsedRealtime());
}

void GLDriver::onFirstRef() {
    ATRACE_CALL();
    status_t err = mSession->linkToComposerDeath(this);
    SLOGE_IF(err, "linkToComposerDeath failed (%s) ", strerror(-err));
    if (err == NO_ERROR) {
        // Load the animation content -- this can be slow (eg 200ms)
        // called before waitForSurfaceFlinger() in main() to avoid wait
        ALOGD("%sGLDriverPreloadTiming start time: %" PRId64 "ms",
                mShuttingDown ? "Shutdown" : "Boot", elapsedRealtime());
    }
}

sp<SurfaceComposerClient> GLDriver::session() const {
    return mSession;
}

void GLDriver::binderDied(const wp<IBinder>&) {
    ATRACE_CALL();
    // woah, surfaceflinger died!
    SLOGD("SurfaceFlinger died, exiting...");

    // calling requestExit() is not enough here because the Surface code
    // might be blocked on a condition variable that will never be updated.
    kill( getpid(), SIGKILL );
    // requestExit();
}

EGLConfig GLDriver::getEglConfig(const EGLDisplay& display) {
    const EGLint attribs[] = {
        EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
        EGL_RED_SIZE,   8,
        EGL_GREEN_SIZE, 8,
        EGL_BLUE_SIZE,  8,
        EGL_ALPHA_SIZE, 8,
        EGL_STENCIL_SIZE, 8,
        EGL_DEPTH_SIZE, 24,
        EGL_SAMPLES, EGL_DONT_CARE,
        EGL_CONFIG_ID, EGL_DONT_CARE,
        EGL_NONE
    };
    EGLint numConfigs;
    EGLConfig config;
    eglChooseConfig(display, attribs, &config, 1, &numConfigs);
    return config;
}

ui::Size GLDriver::limitSurfaceSize(int width, int height) const {
    ui::Size limited(width, height);
    bool wasLimited = false;
    const float aspectRatio = float(width) / float(height);
    if (mMaxWidth != 0 && width > mMaxWidth) {
        limited.height = mMaxWidth / aspectRatio;
        limited.width = mMaxWidth;
        wasLimited = true;
    }
    if (mMaxHeight != 0 && limited.height > mMaxHeight) {
        limited.height = mMaxHeight;
        limited.width = mMaxHeight * aspectRatio;
        wasLimited = true;
    }
    SLOGV_IF(wasLimited, "Surface size has been limited to [%dx%d] from [%dx%d]",
             limited.width, limited.height, width, height);
    return limited;
}

bool GLDriver::init_gl(int display_id) {
    ATRACE_CALL();
    mAssets.addDefaultAssets();
    if (display_id < 0) {
        return false;
    }

    const std::vector<PhysicalDisplayId> ids = SurfaceComposerClient::getPhysicalDisplayIds();
    if (ids.empty()) {
        SLOGE("Failed to get ID for any displays\n");
        return false;
    }
    for (const auto id : ids) {
        SLOGE("Physical display id: %lu", id.value);
    }

    // this system property specifies multi-display IDs to show the boot animation
    // multiple ids can be set with comma (,) as separator, for example:
    // setprop persist.boot.animation.displays 19260422155234049,19261083906282754
    Vector<PhysicalDisplayId> physicalDisplayIds;
    char displayValue[PROPERTY_VALUE_MAX] = "";
    property_get(DISPLAYS_PROP_NAME, displayValue, "");
    bool isValid = displayValue[0] != '\0';
    if (isValid) {
        char *p = displayValue;
        while (*p) {
            if (!isdigit(*p) && *p != ',') {
                isValid = false;
                break;
            }
            p++;
        }
        if (!isValid)
            SLOGE("Invalid syntax for the value of system prop: %s", DISPLAYS_PROP_NAME);
    }
    if (isValid) {
        std::istringstream stream(displayValue);
        for (PhysicalDisplayId id; stream >> id.value; ) {
            physicalDisplayIds.add(id);
            SLOGE("Added display id: %lu", id.value);
            if (stream.peek() == ',')
                stream.ignore();
        }

        // the first specified display id is used to retrieve mDisplayToken
        for (const auto id : physicalDisplayIds) {
            if (std::find(ids.begin(), ids.end(), id) != ids.end()) {
                if (const auto token = SurfaceComposerClient::getPhysicalDisplayToken(id)) {
                    SLOGE("Using display token from display id: %lu", id.value);
                    mDisplayToken = token;
                    break;
                }
            }
        }
    }

    // If the system property is not present or invalid, display 0 is used
    if (mDisplayToken == nullptr) {
        SLOGE("No displays found in the value of system prop: %s", DISPLAYS_PROP_NAME);
        return false;
    }

    DisplayMode displayMode;
    const status_t error =
            SurfaceComposerClient::getActiveDisplayMode(mDisplayToken, &displayMode);
    if (error != NO_ERROR) {
        SLOGE("Error getting active display mode.");
        return false;
    }

    mMaxWidth = android::base::GetIntProperty("ro.surface_flinger.max_graphics_width", 0);
    mMaxHeight = android::base::GetIntProperty("ro.surface_flinger.max_graphics_height", 0);
    ui::Size resolution = displayMode.resolution;
    resolution = limitSurfaceSize(resolution.width, resolution.height);
    // create the native surface
    sp<SurfaceControl> control = session()->createSurface(String8("GLDriver"),
            resolution.getWidth(), resolution.getHeight(), PIXEL_FORMAT_RGBA_8888,
            ISurfaceComposerClient::eNoColorFill);

    SurfaceComposerClient::Transaction t;
    if (isValid) {
        // In the case of multi-display, boot animation shows on the specified displays
        for (const auto id : physicalDisplayIds) {
            if (std::find(ids.begin(), ids.end(), id) != ids.end()) {
                if (const auto token = SurfaceComposerClient::getPhysicalDisplayToken(id)) {
                    SLOGE("Added display id: %lu to layer stack", id.value);

                    ::android::ui::DisplayState displayState;
                    ::android::status_t status =
                            SurfaceComposerClient::getDisplayState(mDisplayToken, &displayState);
                    if (status != ::android::NO_ERROR) {
                        LOG(WARNING) << "Failed to read current mode of the display " << id;
                    }
                    t.setDisplayLayerStack(token, displayState.layerStack);
                    t.setLayerStack(control, displayState.layerStack);
                }
            }
        }
    }

    t
        .setBackgroundColor(control, half3(0.5f, 0.5f, 0.5f), 0.0f, ui::Dataspace::UNKNOWN)
        .setLayer(control, 0x7FFFFFFF)
        .show(control)
        .apply();

    sp<Surface> s = control->getSurface();
    // initialize opengl and egl
    EGLDisplay display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    eglInitialize(display, nullptr, nullptr);
    EGLConfig config = getEglConfig(display);
    EGLSurface surface = eglCreateWindowSurface(display, config, s.get(), nullptr);
    // Initialize egl context with client version number 2.0.
    EGLint contextAttributes[] = {EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE};
    EGLContext context = eglCreateContext(display, config, nullptr, contextAttributes);
    EGLint w, h;
    eglQuerySurface(display, surface, EGL_WIDTH, &w);
    eglQuerySurface(display, surface, EGL_HEIGHT, &h);

    if (eglMakeCurrent(display, surface, surface, context) == EGL_FALSE) {
        SLOGE("Error making context current.");
        return false;
    }

    mDisplay = display;
    mContext = context;
    mSurface = surface;
    mInitWidth = mWidth = w;
    mInitHeight = mHeight = h;
    mFlingerSurfaceControl = control;
    mFlingerSurface = s;
    mTargetInset = -1;

    glClear(GL_COLOR_BUFFER_BIT);

    SLOGE("GL init completed successfully.");
    return true;
}

void GLDriver::projectSceneToWindow() {
    ATRACE_CALL();
    // glViewport(0, 0, mWidth, mHeight);
    // glScissor(0, 0, mWidth, mHeight);
}

void GLDriver::resizeSurface(int newWidth, int newHeight) {
    ATRACE_CALL();
    // We assume this function is called on the animation thread.
    if (newWidth == mWidth && newHeight == mHeight) {
        return;
    }
    SLOGV("Resizing the boot animation surface to %d %d", newWidth, newHeight);

    eglMakeCurrent(mDisplay, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
    eglDestroySurface(mDisplay, mSurface);

    mFlingerSurfaceControl->updateDefaultBufferSize(newWidth, newHeight);
    const auto limitedSize = limitSurfaceSize(newWidth, newHeight);
    mWidth = limitedSize.width;
    mHeight = limitedSize.height;

    EGLConfig config = getEglConfig(mDisplay);
    EGLSurface surface = eglCreateWindowSurface(mDisplay, config, mFlingerSurface.get(), nullptr);
    if (eglMakeCurrent(mDisplay, surface, surface, mContext) == EGL_FALSE) {
        SLOGE("Can't make the new surface current. Error %d", eglGetError());
        return;
    }

    projectSceneToWindow();

    mSurface = surface;
}

void GLDriver::handleViewport(nsecs_t timestep) {
    ATRACE_CALL();
    if (mShuttingDown || !mFlingerSurfaceControl || mTargetInset == 0) {
        return;
    }
    if (mTargetInset < 0) {
        // Poll the amount for the top display inset. This will return -1 until persistent properties
        // have been loaded.
        mTargetInset = android::base::GetIntProperty("persist.sys.displayinset.top",
                -1 /* default */, -1 /* min */, mHeight / 2 /* max */);
    }
    if (mTargetInset <= 0) {
        return;
    }

    if (mCurrentInset < mTargetInset) {
        // After the device boots, the inset will effectively be cropped away. We animate this here.
        float fraction = static_cast<float>(mCurrentInset) / mTargetInset;
        int interpolatedInset = (cosf((fraction + 1) * M_PI) / 2.0f + 0.5f) * mTargetInset;

        SurfaceComposerClient::Transaction()
                .setCrop(mFlingerSurfaceControl, Rect(0, interpolatedInset, mWidth, mHeight))
                .apply();
    } else {
        // At the end of the animation, we switch to the viewport that DisplayManager will apply
        // later. This changes the coordinate system, and means we must move the surface up by
        // the inset amount.
        Rect layerStackRect(0, 0, mWidth, mHeight - mTargetInset);
        Rect displayRect(0, mTargetInset, mWidth, mHeight);

        SurfaceComposerClient::Transaction t;
        t.setPosition(mFlingerSurfaceControl, 0, -mTargetInset)
                .setCrop(mFlingerSurfaceControl, Rect(0, mTargetInset, mWidth, mHeight));
        t.setDisplayProjection(mDisplayToken, ui::ROTATION_0, layerStackRect, displayRect);
        t.apply();

        mTargetInset = mCurrentInset = 0;
    }

    int delta = timestep * mTargetInset / ms2ns(200);
    mCurrentInset += delta;
}

__eglMustCastToProperFunctionPointerType GLDriver::get_process_address(const char *procname) {
    return eglGetProcAddress(procname);
}

bool GLDriver::swap_buffers() {
    return eglSwapBuffers(this->mDisplay, this->mSurface) == EGL_TRUE;
}

bool GLDriver::make_current() {
    return eglMakeCurrent(this->mDisplay, this->mSurface, this->mSurface, this->mContext) == EGL_TRUE;
}

// ---------------------------------------------------------------------------

} // namespace android
