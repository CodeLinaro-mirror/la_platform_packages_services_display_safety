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

#pragma once

#include <androidfw/AssetManager.h>
#include <gui/DisplayEventReceiver.h>
#include <utils/Thread.h>
#include <binder/IBinder.h>
#include <ui/Rotation.h>
#include <EGL/egl.h>
#include <GLES2/gl2.h>
#include <climits>
#include <queue>
#include <stdint.h>
#include <sys/types.h>
#include <vector>

namespace android {

class Surface;
class SurfaceComposerClient;
class SurfaceControl;

// ---------------------------------------------------------------------------

class GLDriver : public IBinder::DeathRecipient
{
public:
    static constexpr int MAX_FADED_FRAMES_COUNT = std::numeric_limits<int>::max();

    explicit GLDriver();
    virtual ~GLDriver();

    // Functions required for creating and using a gl context.
    virtual bool init_gl(int display_id);
    virtual __eglMustCastToProperFunctionPointerType get_process_address(const char *procname);
    virtual bool swap_buffers();
    virtual bool make_current();

private:
    virtual void        onFirstRef();
    virtual void        binderDied(const wp<IBinder>& who);
    sp<SurfaceComposerClient> session() const;

    int displayEventCallback(int fd, int events, void* data);
    bool movie();
    EGLConfig getEglConfig(const EGLDisplay&);
    ui::Size limitSurfaceSize(int width, int height) const;
    void resizeSurface(int newWidth, int newHeight);
    void projectSceneToWindow();
    void handleViewport(nsecs_t timestep);

    std::unique_ptr<DisplayEventReceiver> mDisplayEventReceiver;
    sp<SurfaceComposerClient>       mSession;
    AssetManager mAssets;

    int         mWidth;
    int         mHeight;
    int         mInitWidth;
    int         mInitHeight;
    int         mMaxWidth = 0;
    int         mMaxHeight = 0;
    int         mCurrentInset;
    int         mTargetInset;
    bool        mUseNpotTextures = false;
    EGLDisplay  mDisplay;
    EGLDisplay  mContext;
    EGLDisplay  mSurface;
    sp<IBinder> mDisplayToken;
    sp<SurfaceControl> mFlingerSurfaceControl;
    sp<Surface> mFlingerSurface;
    bool        mShuttingDown;
    bool        mDynamicColorsApplied = false;
    String8     mZipFileName;
    SortedVector<String8> mLoadedFiles;
    GLuint mImageShader;
    GLuint mTextShader;
    GLuint mImageFadeLocation;
    GLuint mImageTextureLocation;
    GLuint mTextCropAreaLocation;
    GLuint mTextTextureLocation;
    GLuint mImageColorProgressLocation;
};

// ---------------------------------------------------------------------------

}; // namespace android
