#include "cusercmd.hpp"

extern "C" void set_motion_sensor_positions(UserData *luaCUserCmd, float *motionSensorPositions)
{
	CUserCmd *cusercmd = (CUserCmd *)luaCUserCmd->data;
	for (int i = 0; i < 20; i++)
	{
		cusercmd->motion_sensor_positions[i][0] = motionSensorPositions[i * 3];
		cusercmd->motion_sensor_positions[i][1] = motionSensorPositions[(i * 3) + 1];
		cusercmd->motion_sensor_positions[i][2] = motionSensorPositions[(i * 3) + 2];
	}
}

// lua_run_cl ORIGINAL_REKINECT_SC = ORIGINAL_REKINECT_SC or hook.GetTable()["StartCommand"]["gmcl_rekinect"] hook.Add("StartCommand", "gmcl_rekinect", function(_, cmd) print(LocalPlayer():MotionSensorPos(0)) if !LocalPlayer():MotionSensorPos(0):IsZero() then ORIGINAL_REKINECT_SC(_, cmd) end end)