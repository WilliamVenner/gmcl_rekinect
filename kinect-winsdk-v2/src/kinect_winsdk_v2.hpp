#include <windows.h>
#include <stdio.h>
#include <Kinect.h>

extern "C"
{
	typedef struct WinSdkKinectV2SkeletonUpdate
	{
		uintptr_t skeletonIndex;
		bool state;
		CameraSpacePoint *skeleton;
	};

	typedef void (*WinSdkKinectV2Callback)(WinSdkKinectV2SkeletonUpdate, void *);
}

class WinSdkKinectV2
{
public:
	WinSdkKinectV2(WinSdkKinectV2Callback callback, void *userdata);
	~WinSdkKinectV2();

	/// Initializes the default Kinect sensor
	HRESULT InitializeDefaultSensor();

	void Run();

	void *m_pCallbackUserData;

private:
	// Current Kinect
	IKinectSensor *m_pKinectSensor;
	ICoordinateMapper *m_pCoordinateMapper;

	// Body reader
	IBodyFrameReader *m_pBodyFrameReader;

	WinSdkKinectV2Callback m_Callback;

	bool m_SkeletonTrackingStates[BODY_COUNT];

	/// Main processing function
	void Update();

	/// Handle new body data
	void ProcessBody(IBody **ppBodies);
};

extern "C" WinSdkKinectV2 *WinSdkKinectV2_Create(WinSdkKinectV2Callback callback, void *userdata, HRESULT *result)
{
	WinSdkKinectV2 *pKinect = new WinSdkKinectV2(callback, userdata);

	*result = pKinect->InitializeDefaultSensor();

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

extern "C" void WinSdkKinectV2_Destroy(WinSdkKinectV2 *pKinect)
{
	if (pKinect != NULL)
	{
		delete pKinect;
	}
}

extern "C" void WinSdkKinectV2_Run(WinSdkKinectV2 *pKinect)
{
	pKinect->Run();
}