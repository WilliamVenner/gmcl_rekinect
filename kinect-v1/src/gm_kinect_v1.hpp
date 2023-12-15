#include <windows.h>
#include <stdio.h>
#include <NuiApi.h>

extern "C"
{
	typedef struct KinectV1SkeletonWithBones
	{
		Vector4 *pos;
		Vector4 *bones;
	};

	typedef union KinectV1Skeleton
	{
		Vector4 *positionOnly;
		KinectV1SkeletonWithBones tracked;
	};

	typedef struct KinectV1SkeletonUpdate
	{
		uintptr_t skeletonIndex;
		NUI_SKELETON_TRACKING_STATE state;
		KinectV1Skeleton skeleton;
	};

	typedef void (*KinectV1Callback)(KinectV1SkeletonUpdate, void *);
}

class KinectV1
{
public:
	KinectV1(KinectV1Callback callback, void *userdata);
	~KinectV1();

	HRESULT Run();

	void *m_pCallbackUserData;

private:
	/// Create the first connected Kinect found
	HRESULT CreateFirstConnected();

	void Update();

	/// Handle new skeleton data
	void ProcessSkeleton();

	KinectV1Callback m_Callback;

	bool m_bSeatedMode;

	// Current Kinect
	INuiSensor *m_pNuiSensor;
	HANDLE m_pSkeletonStreamHandle;
	HANDLE m_hNextSkeletonEvent;

	NUI_SKELETON_TRACKING_STATE m_SkeletonTrackingStates[NUI_SKELETON_COUNT];
};

extern "C" KinectV1 *KinectV1_Create(KinectV1Callback callback, void *userdata)
{
	return new KinectV1(callback, userdata);
}

extern "C" void KinectV1_Destroy(KinectV1 *pKinect, DWORD threadId)
{
	PostThreadMessageW(threadId, WM_QUIT, 0, 0);

	if (pKinect != NULL)
	{
		delete pKinect;
	}
}

extern "C" HRESULT KinectV1_Run(KinectV1 *pKinect)
{
	return pKinect->Run();
}

extern "C" void *KinectV1_UserData(KinectV1 *pKinect)
{
	return pKinect->m_pCallbackUserData;
}
