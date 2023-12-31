#include "kinect_winsdk_v2.hpp"
#include <synchapi.h>

template <class Interface>
inline void SafeRelease(Interface *&pInterfaceToRelease)
{
	if (pInterfaceToRelease != NULL)
	{
		pInterfaceToRelease->Release();
		pInterfaceToRelease = NULL;
	}
}

#define INVALID_WAITABLE_HANDLE reinterpret_cast<WAITABLE_HANDLE>(INVALID_HANDLE_VALUE)

WinSdkKinectV2::WinSdkKinectV2(WinSdkKinectV2Callback callback, void *userdata) : m_pKinectSensor(NULL),
																				  m_pCoordinateMapper(NULL),
																				  m_pBodyFrameReader(NULL),
																				  m_Callback(callback),
																				  m_pCallbackUserData(userdata),
																				  m_AvailablityChangedEvent(INVALID_WAITABLE_HANDLE),
																				  m_BodyFrameArrivedEvent(INVALID_WAITABLE_HANDLE),
																				  m_bAvailable(false)
{
	for (int i = 0; i < BODY_COUNT; ++i)
	{
		m_SkeletonTrackingStates[i] = false;
	}
}

WinSdkKinectV2::~WinSdkKinectV2()
{
	if (m_pKinectSensor && m_AvailablityChangedEvent != INVALID_WAITABLE_HANDLE)
	{
		m_pKinectSensor->UnsubscribeIsAvailableChanged(m_AvailablityChangedEvent);
	}

	if (m_pBodyFrameReader && m_BodyFrameArrivedEvent != INVALID_WAITABLE_HANDLE)
	{
		m_pBodyFrameReader->UnsubscribeFrameArrived(m_BodyFrameArrivedEvent);
	}

	// done with body frame reader
	SafeRelease(m_pBodyFrameReader);

	// done with coordinate mapper
	SafeRelease(m_pCoordinateMapper);

	// close the Kinect Sensor
	if (m_pKinectSensor)
	{
		m_pKinectSensor->Close();
	}

	SafeRelease(m_pKinectSensor);
}

HRESULT WinSdkKinectV2::Run()
{
	MSG msg = {0};

	const int eventCount = 2;
	HANDLE hEvents[eventCount];

	// Main message loop
	while (WM_QUIT != msg.message)
	{
		hEvents[0] = reinterpret_cast<HANDLE>(m_BodyFrameArrivedEvent);
		hEvents[1] = reinterpret_cast<HANDLE>(m_AvailablityChangedEvent);

		// Check to see if we have either a message (by passing in QS_ALLEVENTS)
		// Or a Kinect event (hEvents)
		// Update() will check for Kinect events individually, in case more than one are signalled
		DWORD event = MsgWaitForMultipleObjects(eventCount, hEvents, FALSE, INFINITE, QS_ALLINPUT);

		if (event == WAIT_OBJECT_0 + eventCount)
		{
			if (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE))
			{
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			}
		}
		else if (event >= WAIT_OBJECT_0 && event < WAIT_OBJECT_0 + eventCount)
		{
			Update(event - WAIT_OBJECT_0);
		}
		else
		{
			return HRESULT_FROM_WIN32(GetLastError());
		}
	}

	return S_OK;
}

void WinSdkKinectV2::BodyFrameArrived()
{
	if (m_pKinectSensor)
	{
		// I don't understand how they managed to mess this part of the SDK up so badly, but this seems to be a hacky fix for this event just not working properly...
		BOOLEAN bAvailable = FALSE;
		if (SUCCEEDED(m_pKinectSensor->get_IsAvailable(&bAvailable)) && m_bAvailable.load(std::memory_order_relaxed) != !!bAvailable)
		{
			AvailableChanged();
		}
	}

	if (!m_pBodyFrameReader)
	{
		return;
	}

	IBodyFrame *pBodyFrame = NULL;

	HRESULT hr = m_pBodyFrameReader->AcquireLatestFrame(&pBodyFrame);

	if (SUCCEEDED(hr) && pBodyFrame)
	{
		IBody *ppBodies[BODY_COUNT] = {0};

		hr = pBodyFrame->GetAndRefreshBodyData(_countof(ppBodies), ppBodies);

		if (SUCCEEDED(hr))
		{
			ProcessBody(ppBodies);
		}

		for (int i = 0; i < _countof(ppBodies); ++i)
		{
			SafeRelease(ppBodies[i]);
		}
	}

	SafeRelease(pBodyFrame);
}

void WinSdkKinectV2::AvailableChanged()
{
	if (!m_pKinectSensor)
	{
		return;
	}

	IIsAvailableChangedEventArgs *pAvailableChangedEvent = NULL;

	HRESULT hr = m_pKinectSensor->GetIsAvailableChangedEventData(m_AvailablityChangedEvent, &pAvailableChangedEvent);

	if (SUCCEEDED(hr) && pAvailableChangedEvent)
	{
		BOOLEAN bAvailable = FALSE;
		hr = pAvailableChangedEvent->get_IsAvailable(&bAvailable);

		if (SUCCEEDED(hr))
		{
			m_bAvailable.store(bAvailable, std::memory_order_release);

			if (!bAvailable)
			{
				for (int i = 0; i < BODY_COUNT; ++i)
				{
					bool trackingStateChanged = m_SkeletonTrackingStates[i] != false;

					m_SkeletonTrackingStates[i] = false;

					if (trackingStateChanged)
					{
						m_Callback({(uintptr_t)i, NULL}, m_pCallbackUserData);
					}
				}
			}
		}
	}

	SafeRelease(pAvailableChangedEvent);

	// Not sure why this is needed, but without it, the event is never "re-armed"
	m_pKinectSensor->UnsubscribeIsAvailableChanged(m_AvailablityChangedEvent);
	m_pKinectSensor->SubscribeIsAvailableChanged(&m_AvailablityChangedEvent);
}

void WinSdkKinectV2::Update(DWORD event)
{
	switch (event)
	{
	case 0:
		BodyFrameArrived();
		break;

	case 1:
		AvailableChanged();
		break;
	}
}

HRESULT WinSdkKinectV2::InitializeDefaultSensor()
{
	HRESULT hr = GetDefaultKinectSensor(&m_pKinectSensor);
	if (FAILED(hr))
	{
		return hr;
	}

	if (m_pKinectSensor)
	{
		// Initialize the Kinect and get coordinate mapper and the body reader
		IBodyFrameSource *pBodyFrameSource = NULL;

		hr = m_pKinectSensor->SubscribeIsAvailableChanged(&m_AvailablityChangedEvent);

		if (SUCCEEDED(hr))
		{
			hr = m_pKinectSensor->Open();
		}

		if (SUCCEEDED(hr))
		{
			hr = m_pKinectSensor->get_CoordinateMapper(&m_pCoordinateMapper);
		}

		if (SUCCEEDED(hr))
		{
			hr = m_pKinectSensor->get_BodyFrameSource(&pBodyFrameSource);
		}

		if (SUCCEEDED(hr))
		{
			hr = pBodyFrameSource->SubscribeFrameCaptured(&m_BodyFrameArrivedEvent);
		}

		if (SUCCEEDED(hr))
		{
			hr = pBodyFrameSource->OpenReader(&m_pBodyFrameReader);
		}

		SafeRelease(pBodyFrameSource);
	}

	if (!m_pKinectSensor)
	{
		return E_FAIL;
	}

	return hr;
}

void WinSdkKinectV2::ProcessBody(IBody **ppBodies)
{
	if (m_pCoordinateMapper)
	{
		for (int i = 0; i < BODY_COUNT; ++i)
		{
			IBody *pBody = ppBodies[i];
			if (pBody)
			{
				BOOLEAN bTracked = false;
				HRESULT hr = pBody->get_IsTracked(&bTracked);

				bTracked = SUCCEEDED(hr) && bTracked;

				const bool trackingStateChanged = bTracked != m_SkeletonTrackingStates[i];

				m_SkeletonTrackingStates[i] = bTracked;

				if (!bTracked)
				{
					if (trackingStateChanged)
					{
						m_Callback({(uintptr_t)i, NULL}, m_pCallbackUserData);
					}
				}
				else
				{
					Joint joints[JointType_Count];
					hr = pBody->GetJoints(_countof(joints), joints);

					if (SUCCEEDED(hr))
					{
						CameraSpacePoint positions[JointType_Count];

						for (int j = 0; j < _countof(joints); ++j)
						{
							positions[j] = joints[j].Position;
						}

						m_Callback({(uintptr_t)i, positions}, m_pCallbackUserData);
					}
				}
			}
		}
	}
}