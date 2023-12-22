#include "kinect_winsdk_v1.hpp"

template <class Interface>
inline void SafeRelease(Interface *&pInterfaceToRelease)
{
	if (pInterfaceToRelease != NULL)
	{
		pInterfaceToRelease->Release();
		pInterfaceToRelease = NULL;
	}
}

WinSdkKinectV1::WinSdkKinectV1(
	WinSdkKinectV1Callback callback, void *userdata) : m_hNextSkeletonEvent(INVALID_HANDLE_VALUE),
													   m_pNuiSensor(NULL),
													   m_Callback(callback),
													   m_pCallbackUserData(userdata)
{
	for (int i = 0; i < NUI_SKELETON_COUNT; ++i)
	{
		m_SkeletonTrackingStates[i] = false;
	}
}

WinSdkKinectV1::~WinSdkKinectV1()
{
	if (m_pNuiSensor)
	{
		m_pNuiSensor->NuiShutdown();
	}

	if (m_hNextSkeletonEvent && (m_hNextSkeletonEvent != INVALID_HANDLE_VALUE))
	{
		CloseHandle(m_hNextSkeletonEvent);
	}

	SafeRelease(m_pNuiSensor);
}

void WinSdkKinectV1::Run()
{
	MSG msg = {0};

	const int eventCount = 1;
	HANDLE hEvents[eventCount];

	// Main message loop
	while (WM_QUIT != msg.message)
	{
		hEvents[0] = m_hNextSkeletonEvent;

		// Check to see if we have either a message (by passing in QS_ALLEVENTS)
		// Or a Kinect event (hEvents)
		// Update() will check for Kinect events individually, in case more than one are signalled
		DWORD event = MsgWaitForMultipleObjects(eventCount, hEvents, FALSE, INFINITE, QS_ALLINPUT);

		// Explicitly check the Kinect frame event since MsgWaitForMultipleObjects
		// can return for other reasons even though it is signaled.
		Update(WAIT_OBJECT_0 - event);

		while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE))
		{
			TranslateMessage(&msg);
			DispatchMessageW(&msg);
		}
	}
}

void WinSdkKinectV1::Update(DWORD event)
{
	if (event == 0)
	{
		ProcessSkeleton();
	}
}

void WinSdkKinectV1::DeviceStatusChanged(HRESULT hrStatus, const OLECHAR *instanceName, const OLECHAR *uniqueDeviceName, void *pUserData)
{
	WinSdkKinectV1 *kinect = (WinSdkKinectV1 *)pUserData;

	if (kinect->m_pNuiSensor)
	{
		kinect->m_pNuiSensor->NuiShutdown();
		kinect->m_pNuiSensor->Release();
		kinect->m_pNuiSensor = NULL;

		for (int i = 0; i < NUI_SKELETON_COUNT; ++i)
		{
			bool trackingStateChanged = kinect->m_SkeletonTrackingStates[i] != false;

			kinect->m_SkeletonTrackingStates[i] = false;

			if (trackingStateChanged)
			{
				kinect->m_Callback({(uintptr_t)i, NULL}, kinect->m_pCallbackUserData);
			}
		}
	}

	if (SUCCEEDED(hrStatus))
	{
		INuiSensor *pNuiSensor;
		HRESULT hr = NuiCreateSensorById(instanceName, &pNuiSensor);

		if (SUCCEEDED(hr))
		{
			// Get the status of the sensor, and if connected, then we can initialize it
			hr = pNuiSensor->NuiStatus();

			if (SUCCEEDED(hr))
			{
				// Initialize the Kinect and specify that we'll be using skeleton
				hr = pNuiSensor->NuiInitialize(NUI_INITIALIZE_FLAG_USES_SKELETON);
			}

			if (SUCCEEDED(hr))
			{
				// Open a skeleton stream to receive skeleton data
				hr = pNuiSensor->NuiSkeletonTrackingEnable(kinect->m_hNextSkeletonEvent, 0);
			}

			if (SUCCEEDED(hr))
			{
				kinect->m_pNuiSensor = pNuiSensor;
			}
			else
			{
				// This sensor wasn't OK, so release it since we're not using it
				pNuiSensor->Release();
			}
		}
	}
}

HRESULT WinSdkKinectV1::MonitorSensors()
{
	// Create an event that will be signaled when skeleton data is available
	m_hNextSkeletonEvent = CreateEventW(NULL, TRUE, FALSE, NULL);

	NuiSetDeviceStatusCallback(WinSdkKinectV1::DeviceStatusChanged, this);

	int iSensorCount = 0;
	HRESULT hr = NuiGetSensorCount(&iSensorCount);
	if (FAILED(hr))
	{
		return hr;
	}

	INuiSensor *pNuiSensor;

	// Look at each Kinect sensor
	for (int i = 0; i < iSensorCount; ++i)
	{
		// Create the sensor so we can check status, if we can't create it, move on to the next
		hr = NuiCreateSensorByIndex(i, &pNuiSensor);
		if (FAILED(hr))
		{
			continue;
		}

		// Get the status of the sensor, and if connected, then we can initialize it
		hr = pNuiSensor->NuiStatus();
		if (S_OK == hr)
		{
			m_pNuiSensor = pNuiSensor;
			break;
		}

		// This sensor wasn't OK, so release it since we're not using it
		pNuiSensor->Release();
	}

	if (NULL != m_pNuiSensor)
	{
		// Initialize the Kinect and specify that we'll be using skeleton
		hr = m_pNuiSensor->NuiInitialize(NUI_INITIALIZE_FLAG_USES_SKELETON);

		if (SUCCEEDED(hr))
		{
			// Open a skeleton stream to receive skeleton data
			hr = m_pNuiSensor->NuiSkeletonTrackingEnable(m_hNextSkeletonEvent, 0);
		}
	}

	return hr;
}

/// Handle new skeleton data
void WinSdkKinectV1::ProcessSkeleton()
{
	if (NULL == m_pNuiSensor)
	{
		return;
	}

	NUI_SKELETON_FRAME skeletonFrame = {0};

	const HRESULT hr = m_pNuiSensor->NuiSkeletonGetNextFrame(0, &skeletonFrame);
	if (FAILED(hr))
	{
		return;
	}

	// smooth out the skeleton data
	m_pNuiSensor->NuiTransformSmooth(&skeletonFrame, NULL);

	for (int i = 0; i < NUI_SKELETON_COUNT; ++i)
	{
		bool bIsTracked = skeletonFrame.SkeletonData[i].eTrackingState == NUI_SKELETON_TRACKED;

		const bool trackingStateChanged = bIsTracked != m_SkeletonTrackingStates[i];

		m_SkeletonTrackingStates[i] = bIsTracked;

		if (bIsTracked)
		{
			m_Callback({(uintptr_t)i, skeletonFrame.SkeletonData[i].SkeletonPositions}, m_pCallbackUserData);
		}
		else if (trackingStateChanged)
		{
			m_Callback({(uintptr_t)i, NULL}, m_pCallbackUserData);
		}
	}
}
