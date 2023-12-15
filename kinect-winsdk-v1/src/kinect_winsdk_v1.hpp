#include <windows.h>
#include <stdio.h>
#include <NuiApi.h>

extern "C"
{
	typedef struct WinSdkKinectV1SkeletonWithBones
	{
		Vector4 *pos;
		Vector4 *bones;
	};

	typedef union WinSdkKinectV1Skeleton
	{
		Vector4 *positionOnly;
		WinSdkKinectV1SkeletonWithBones tracked;
	};

	typedef struct WinSdkKinectV1SkeletonUpdate
	{
		uintptr_t skeletonIndex;
		NUI_SKELETON_TRACKING_STATE state;
		WinSdkKinectV1Skeleton skeleton;
	};

	typedef void (*WinSdkKinectV1Callback)(WinSdkKinectV1SkeletonUpdate, void *);
}

class WinSdkKinectV1
{
public:
	WinSdkKinectV1(WinSdkKinectV1Callback callback, void *userdata);
	~WinSdkKinectV1();

	HRESULT Run();

	void *m_pCallbackUserData;

private:
	/// Create the first connected Kinect found
	HRESULT CreateFirstConnected();

	void Update();

	/// Handle new skeleton data
	void ProcessSkeleton();

	WinSdkKinectV1Callback m_Callback;

	bool m_bSeatedMode;

	// Current Kinect
	INuiSensor *m_pNuiSensor;
	HANDLE m_pSkeletonStreamHandle;
	HANDLE m_hNextSkeletonEvent;

	NUI_SKELETON_TRACKING_STATE m_SkeletonTrackingStates[NUI_SKELETON_COUNT];
};

extern "C" WinSdkKinectV1 *WinSdkKinectV1_Create(WinSdkKinectV1Callback callback, void *userdata)
{
	return new WinSdkKinectV1(callback, userdata);
}

extern "C" void WinSdkKinectV1_Destroy(WinSdkKinectV1 *pKinect, DWORD threadId)
{
	PostThreadMessageW(threadId, WM_QUIT, 0, 0);

	if (pKinect != NULL)
	{
		delete pKinect;
	}
}

extern "C" HRESULT WinSdkKinectV1_Run(WinSdkKinectV1 *pKinect)
{
	return pKinect->Run();
}