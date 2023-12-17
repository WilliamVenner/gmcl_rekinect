#pragma once
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
	int command_number;
	int tick_count;
	QAngle viewangles;
	float forwardmove;
	float sidemove;
	float upmove;
	int buttons;
	uint8_t impulse;
	int weaponselect;
	int weaponsubtype;
	int random_seed;
	int server_random_seed;
	short mousedx;
	short mousedy;
	bool hasbeenpredicted;
	uint8_t buttons_pressed[5];
	// int8_t scroll_wheel_speed;
	bool world_clicking;
	float world_click_direction[2];
	bool is_typing;
	vec3_t motion_sensor_positions[20]; // kinect stuff
	bool forced;
};

extern "C" void set_motion_sensor_positions(UserData *luaCUserCmd, float *motionSensorPositions);