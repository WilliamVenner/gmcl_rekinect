#include "kinect_winsdk_v2.hpp"

template <class Interface>
inline void SafeRelease(Interface *&pInterfaceToRelease)
{
	if (pInterfaceToRelease != NULL)
	{
		pInterfaceToRelease->Release();
		pInterfaceToRelease = NULL;
	}
}

WinSdkKinectV2::WinSdkKinectV2(WinSdkKinectV2Callback callback, void *userdata) : m_pKinectSensor(NULL),
																				  m_pCoordinateMapper(NULL),
																				  m_pBodyFrameReader(NULL),
																				  m_Callback(callback),
																				  m_pCallbackUserData(userdata)
{
}

WinSdkKinectV2::~WinSdkKinectV2()
{
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

void WinSdkKinectV2::Run()
{
	MSG msg = {0};

	// Main message loop
	while (WM_QUIT != msg.message)
	{
		Update();

		while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE))
		{
			TranslateMessage(&msg);
			DispatchMessageW(&msg);
		}
	}
}

void WinSdkKinectV2::Update()
{
	if (!m_pBodyFrameReader)
	{
		return;
	}

	IBodyFrame *pBodyFrame = NULL;

	HRESULT hr = m_pBodyFrameReader->AcquireLatestFrame(&pBodyFrame);

	if (SUCCEEDED(hr))
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

		BOOLEAN available = FALSE;
		hr = m_pKinectSensor->get_IsAvailable(&available);

		if (SUCCEEDED(hr) && !available)
		{
			hr = E_FAIL;
		}

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
						m_Callback({(uintptr_t)i, false}, m_pCallbackUserData);
					}
				}
				else
				{
					Joint joints[JointType_Count];
					CameraSpacePoint skeleton[JointType_Count] = {0};

					hr = pBody->GetJoints(_countof(joints), joints);
					if (SUCCEEDED(hr))
					{
						for (int j = 0; j < _countof(joints); ++j)
						{
							skeleton[j] = joints[j].Position;
						}

						m_Callback({(uintptr_t)i, true, skeleton}, m_pCallbackUserData);
					}
				}
			}
		}
	}
}