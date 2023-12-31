#include <windows.h>
#include <stdio.h>
#include <Kinect.h>
#include <atomic>

extern "C"
{
	struct WinSdkKinectV2SkeletonUpdate
	{
		uintptr_t skeletonIndex;
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

	HRESULT Run();

	void *m_pCallbackUserData;
	std::atomic<bool> m_bAvailable;

private:
	// Current Kinect
	IKinectSensor *m_pKinectSensor;
	ICoordinateMapper *m_pCoordinateMapper;
	WAITABLE_HANDLE m_AvailablityChangedEvent;
	WAITABLE_HANDLE m_BodyFrameArrivedEvent;

	// Body reader
	IBodyFrameReader *m_pBodyFrameReader;

	WinSdkKinectV2Callback m_Callback;

	BOOLEAN m_SkeletonTrackingStates[BODY_COUNT];

	/// Main processing function
	void Update(DWORD event);

	/// Handle new body data
	void ProcessBody(IBody **ppBodies);

	// Event handlers
	void AvailableChanged();
	void BodyFrameArrived();
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

extern "C" HRESULT WinSdkKinectV2_Run(WinSdkKinectV2 *pKinect)
{
	return pKinect->Run();
}

extern "C" bool WinSdkKinectV2_Available(WinSdkKinectV2 *pKinect)
{
	return pKinect->m_bAvailable.load(std::memory_order_acquire);
}