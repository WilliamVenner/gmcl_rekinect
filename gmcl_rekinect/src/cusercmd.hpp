#pragma once
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

typedef float vec_t;
typedef float vec3_t[3];

struct UserData
{
	void *data;
	unsigned char type;
};

class QAngle
{
public:
	vec_t x, y, z;
};

class CUserCmd
{
public:
	int command_number;					// 0  (crc)
	int tick_count;						// 4  (crc)
	QAngle viewangles;					// 8  (crc)
	float forwardmove;					// 20 (crc)
	float sidemove;						// 24 (crc)
	float upmove;						// 28 (crc)
	int buttons;						// 32 (crc)
	int weaponselect;					// 36 (crc)
	int weaponsubtype;					// 40 (crc)
	int random_seed;					// 44 (crc)
	short mousedx;						// 48 (crc)
	short mousedy;						// 50 (crc)
	bool hasbeenpredicted;				// 52
	uint8_t impulse;					// 53 (crc)
	bool is_typing;						// 54, networked by ReadUsercmd
	bool forced;						// 55, CUserCmd:IsForced reads server 0x3b
	uint8_t buttons_pressed[5];			// 56
	int8_t scroll_wheel_speed;			// 61 (crc), CUserCmd:GetMouseWheel sign-extends it
	bool world_clicking;				// 62, 1 byte of padding follows
	vec3_t world_click_direction;		// 64, a Vector - three floats, not two
	vec3_t motion_sensor_positions[20]; // 76, kinect stuff
};

#define CUSERCMD_ASSERT_OFFSET(member, offset) \
	static_assert(offsetof(CUserCmd, member) == offset, "CUserCmd::" #member " is not at offset " #offset)

CUSERCMD_ASSERT_OFFSET(command_number, 0);
CUSERCMD_ASSERT_OFFSET(tick_count, 4);
CUSERCMD_ASSERT_OFFSET(viewangles, 8);
CUSERCMD_ASSERT_OFFSET(forwardmove, 20);
CUSERCMD_ASSERT_OFFSET(sidemove, 24);
CUSERCMD_ASSERT_OFFSET(upmove, 28);
CUSERCMD_ASSERT_OFFSET(buttons, 32);
CUSERCMD_ASSERT_OFFSET(weaponselect, 36);
CUSERCMD_ASSERT_OFFSET(weaponsubtype, 40);
CUSERCMD_ASSERT_OFFSET(random_seed, 44);
CUSERCMD_ASSERT_OFFSET(mousedx, 48);
CUSERCMD_ASSERT_OFFSET(mousedy, 50);
CUSERCMD_ASSERT_OFFSET(impulse, 53);
CUSERCMD_ASSERT_OFFSET(scroll_wheel_speed, 61);
CUSERCMD_ASSERT_OFFSET(world_click_direction, 64);
CUSERCMD_ASSERT_OFFSET(motion_sensor_positions, 76);
static_assert(sizeof(CUserCmd) == 316, "CUserCmd is not 316 bytes");

#undef CUSERCMD_ASSERT_OFFSET

extern "C" void set_motion_sensor_positions(UserData *luaCUserCmd, float *motionSensorPositions);