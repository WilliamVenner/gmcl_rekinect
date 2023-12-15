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
													   m_pSkeletonStreamHandle(INVALID_HANDLE_VALUE),
													   m_bSeatedMode(false),
													   m_pNuiSensor(NULL),
													   m_Callback(callback),
													   m_pCallbackUserData(userdata)
{
	for (int i = 0; i < NUI_SKELETON_COUNT; ++i)
	{
		m_SkeletonTrackingStates[i] = NUI_SKELETON_NOT_TRACKED;
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
		MsgWaitForMultipleObjects(eventCount, hEvents, FALSE, INFINITE, QS_ALLINPUT);

		// Explicitly check the Kinect frame event since MsgWaitForMultipleObjects
		// can return for other reasons even though it is signaled.
		Update();

		while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE))
		{
			TranslateMessage(&msg);
			DispatchMessageW(&msg);
		}
	}
}

void WinSdkKinectV1::Update()
{
	if (NULL == m_pNuiSensor)
	{
		return;
	}

	// Wait for 0ms, just quickly test if it is time to process a skeleton
	if (WAIT_OBJECT_0 == WaitForSingleObject(m_hNextSkeletonEvent, 0))
	{
		ProcessSkeleton();
	}
}

/// Create the first connected Kinect found
HRESULT WinSdkKinectV1::CreateFirstConnected()
{
	INuiSensor *pNuiSensor;

	int iSensorCount = 0;
	HRESULT hr = NuiGetSensorCount(&iSensorCount);
	if (FAILED(hr))
	{
		return hr;
	}

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
			// Create an event that will be signaled when skeleton data is available
			m_hNextSkeletonEvent = CreateEventW(NULL, TRUE, FALSE, NULL);

			// Open a skeleton stream to receive skeleton data
			hr = m_pNuiSensor->NuiSkeletonTrackingEnable(m_hNextSkeletonEvent, 0);
		}
	}

	if (NULL == m_pNuiSensor)
	{
		return E_FAIL;
	}

	return hr;
}

/// Handle new skeleton data
void WinSdkKinectV1::ProcessSkeleton()
{
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
		const NUI_SKELETON_TRACKING_STATE trackingState = skeletonFrame.SkeletonData[i].eTrackingState;

		const bool trackingStateChanged = trackingState != m_SkeletonTrackingStates[i];

		m_SkeletonTrackingStates[i] = trackingState;

		if (NUI_SKELETON_NOT_TRACKED == trackingState)
		{
			if (trackingStateChanged)
			{
				m_Callback({(uintptr_t)i, NUI_SKELETON_NOT_TRACKED}, m_pCallbackUserData);
			}
		}
		else
		{
			if (NUI_SKELETON_TRACKED == trackingState)
			{
				WinSdkKinectV1Skeleton skeleton;
				skeleton.tracked = {&skeletonFrame.SkeletonData[i].Position, skeletonFrame.SkeletonData[i].SkeletonPositions};
				m_Callback({(uintptr_t)i, NUI_SKELETON_TRACKED, skeleton}, m_pCallbackUserData);
			}
			else if (NUI_SKELETON_POSITION_ONLY == trackingState)
			{
				WinSdkKinectV1Skeleton skeleton;
				skeleton.positionOnly = &skeletonFrame.SkeletonData[i].Position;
				m_Callback({(uintptr_t)i, NUI_SKELETON_POSITION_ONLY, skeleton}, m_pCallbackUserData);
			}
		}
	}
}
