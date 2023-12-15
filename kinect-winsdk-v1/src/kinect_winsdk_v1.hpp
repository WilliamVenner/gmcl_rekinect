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

	void Run();

	/// Create the first connected Kinect found
	HRESULT CreateFirstConnected();

	void *m_pCallbackUserData;

private:
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

extern "C" WinSdkKinectV1 *WinSdkKinectV1_Create(WinSdkKinectV1Callback callback, void *userdata, HRESULT *result)
{
	WinSdkKinectV1 *pKinect = new WinSdkKinectV1(callback, userdata);

	*result = pKinect->CreateFirstConnected();

	if (FAILED(*result))
	{
		return NULL;
	}
	else
	{
		*result = S_OK;
		return pKinect;
	}
}

extern "C" void WinSdkKinectV1_Destroy(WinSdkKinectV1 *pKinect)
{
	if (pKinect != NULL)
	{
		delete pKinect;
	}
}

extern "C" void WinSdkKinectV1_Run(WinSdkKinectV1 *pKinect)
{
	pKinect->Run();
}