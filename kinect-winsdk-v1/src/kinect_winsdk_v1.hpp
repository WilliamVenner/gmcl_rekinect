#include <windows.h>
#include <stdio.h>
#include <NuiApi.h>
#include <atomic>

extern "C"
{
	struct WinSdkKinectV1SkeletonUpdate
	{
		uintptr_t skeletonIndex;
		Vector4 *bones;
	};

	typedef void (*WinSdkKinectV1Callback)(WinSdkKinectV1SkeletonUpdate, void *);
}

class WinSdkKinectV1
{
public:
	WinSdkKinectV1(WinSdkKinectV1Callback callback, void *userdata);
	~WinSdkKinectV1();

	HRESULT Run();

	HRESULT MonitorSensors();

	void *m_pCallbackUserData;
	std::atomic<bool> m_bAvailable;

private:
	void Update(DWORD event);

	/// Handle new skeleton data
	void ProcessSkeleton();

	static void CALLBACK DeviceStatusChanged(HRESULT hrStatus, const OLECHAR *instanceName, const OLECHAR *uniqueDeviceName, void *pUserData);

	WinSdkKinectV1Callback m_Callback;

	// Current Kinect
	INuiSensor *m_pNuiSensor;
	HANDLE m_hNextSkeletonEvent;

	bool m_SkeletonTrackingStates[NUI_SKELETON_COUNT];
};

extern "C" WinSdkKinectV1 *WinSdkKinectV1_Create(WinSdkKinectV1Callback callback, void *userdata, HRESULT *result)
{
	WinSdkKinectV1 *kinect = new WinSdkKinectV1(callback, userdata);

	if (kinect)
	{
		*result = kinect->MonitorSensors();
	}
	else
	{
		*result = E_FAIL;
	}

	return kinect;
}

extern "C" void WinSdkKinectV1_Destroy(WinSdkKinectV1 *pKinect)
{
	if (pKinect != NULL)
	{
		delete pKinect;
	}
}

extern "C" HRESULT WinSdkKinectV1_Run(WinSdkKinectV1 *pKinect)
{
	return pKinect->Run();
}

extern "C" bool WinSdkKinectV1_Available(WinSdkKinectV1 *pKinect)
{
	return pKinect->m_bAvailable.load(std::memory_order_acquire);
}