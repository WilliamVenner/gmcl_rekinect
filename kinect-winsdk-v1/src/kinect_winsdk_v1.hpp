#include <windows.h>
#include <stdio.h>
#include <NuiApi.h>

extern "C"
{
	typedef struct WinSdkKinectV1SkeletonUpdate
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

	void Run();

	HRESULT MonitorSensors();

	void *m_pCallbackUserData;

private:
	void Update(DWORD event);

	/// Handle new skeleton data
	void ProcessSkeleton();

	static void DeviceStatusChanged(HRESULT hrStatus, const OLECHAR *instanceName, const OLECHAR *uniqueDeviceName, void *pUserData);

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

extern "C" void WinSdkKinectV1_Run(WinSdkKinectV1 *pKinect)
{
	pKinect->Run();
}