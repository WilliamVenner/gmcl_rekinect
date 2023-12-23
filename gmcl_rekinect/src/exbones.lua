local function init(serverSupportsExtendedBones)
	if serverSupportsExtendedBones then
		SENSORBONE.SPINE_BASE = 20
		SENSORBONE.NECK = 21
		SENSORBONE.SPINE_SHOULDER = 22
		SENSORBONE.HAND_TIP_LEFT = 23
		SENSORBONE.THUMB_LEFT = 24
		SENSORBONE.HAND_TIP_RIGHT = 25
		SENSORBONE.THUMB_RIGHT = 26

		if SERVER then
			util.AddNetworkString("gmcl_rekinect_extended_bones")
		end

		local playerExtendedBones = {}
		motionsensor.RekinectExtendedBonesRegistry = playerExtendedBones

		net.Receive("gmcl_rekinect_extended_bones", function(len, ply)
			if len == 0 then return end -- Checking if server supports extended bones
			local id = CLIENT and net.ReadUInt(32) or ply:UserID()
			local cmdNumber = net.ReadUInt(32)
			local clear = net.ReadBool()

			if not clear then
				playerExtendedBones[id] = playerExtendedBones[id] or {}
				local playerExtendedBones = playerExtendedBones[id]
				if playerExtendedBones[cmdNumber] and cmdNumber <= playerExtendedBones[cmdNumber] then return end -- Out of order packet, ignore
				playerExtendedBones.cmdNumber = cmdNumber
				playerExtendedBones[SENSORBONE.SPINE_BASE] = net.ReadVector()
				playerExtendedBones[SENSORBONE.NECK] = net.ReadVector()
				playerExtendedBones[SENSORBONE.SPINE_SHOULDER] = net.ReadVector()
				playerExtendedBones[SENSORBONE.HAND_TIP_LEFT] = net.ReadVector()
				playerExtendedBones[SENSORBONE.THUMB_LEFT] = net.ReadVector()
				playerExtendedBones[SENSORBONE.HAND_TIP_RIGHT] = net.ReadVector()
				playerExtendedBones[SENSORBONE.THUMB_RIGHT] = net.ReadVector()

				if SERVER then
					net.Start("gmcl_rekinect_extended_bones", true)
					net.WriteUInt(id, 32)
					net.WriteUInt(cmdNumber, 32)
					net.WriteBool(false)
					net.WriteVector(playerExtendedBones[SENSORBONE.SPINE_BASE])
					net.WriteVector(playerExtendedBones[SENSORBONE.NECK])
					net.WriteVector(playerExtendedBones[SENSORBONE.SPINE_SHOULDER])
					net.WriteVector(playerExtendedBones[SENSORBONE.HAND_TIP_LEFT])
					net.WriteVector(playerExtendedBones[SENSORBONE.THUMB_LEFT])
					net.WriteVector(playerExtendedBones[SENSORBONE.HAND_TIP_RIGHT])
					net.WriteVector(playerExtendedBones[SENSORBONE.THUMB_RIGHT])
					net.Broadcast()
				end
			else
				if playerExtendedBones[id] and playerExtendedBones[id][cmdNumber] and cmdNumber <= playerExtendedBones[id][cmdNumber] then return end -- Out of order packet, ignore

				playerExtendedBones[id] = {
					cmdNumber = cmdNumber
				}

				if SERVER then
					net.Start("gmcl_rekinect_extended_bones")
					net.WriteUInt(id, 32)
					net.WriteUInt(cmdNumber, 32)
					net.WriteBool(true)
					net.Broadcast()
				end
			end
		end)

		gameevent.Listen("player_disconnect")

		hook.Add("player_disconnect", "gmcl_rekinect_extended_bones", function(_, __, ___, id)
			playerExtendedBones[id] = nil
		end)

		local PLAYER = FindMetaTable("Player")
		local MotionSensorPos = PLAYER.MotionSensorPos

		function PLAYER:MotionSensorPos(bone)
			local exBone

			if bone >= SENSORBONE.SPINE_BASE and bone <= SENSORBONE.THUMB_RIGHT then
				local id = self:UserID()
				exBone = playerExtendedBones[id] and playerExtendedBones[id][bone] or Vector()
			end

			if exBone then
				return exBone
			else
				return MotionSensorPos(self, bone)
			end
		end

		if CLIENT then
			chat.AddText(Color(0, 255, 0), "gmcl_rekinect: Server supports Xbox One Kinect extended bones.")
			local GetSkeleton = motionsensor.GetSkeleton

			function motionsensor.GetSkeleton()
				if not motionsensor.IsActive() then return nil end
				local ply = LocalPlayer()
				local MotionSensorPos = ply.MotionSensorPos
				local skeleton

				if GetSkeleton then
					skeleton = GetSkeleton()
				else
					skeleton = {
						[SENSORBONE.SHOULDER_RIGHT] = MotionSensorPos(ply, SENSORBONE.SHOULDER_RIGHT),
						[SENSORBONE.SHOULDER_LEFT] = MotionSensorPos(ply, SENSORBONE.SHOULDER_LEFT),
						[SENSORBONE.HIP] = MotionSensorPos(ply, SENSORBONE.HIP),
						[SENSORBONE.ELBOW_RIGHT] = MotionSensorPos(ply, SENSORBONE.ELBOW_RIGHT),
						[SENSORBONE.KNEE_RIGHT] = MotionSensorPos(ply, SENSORBONE.KNEE_RIGHT),
						[SENSORBONE.WRIST_RIGHT] = MotionSensorPos(ply, SENSORBONE.WRIST_RIGHT),
						[SENSORBONE.ANKLE_LEFT] = MotionSensorPos(ply, SENSORBONE.ANKLE_LEFT),
						[SENSORBONE.FOOT_LEFT] = MotionSensorPos(ply, SENSORBONE.FOOT_LEFT),
						[SENSORBONE.WRIST_LEFT] = MotionSensorPos(ply, SENSORBONE.WRIST_LEFT),
						[SENSORBONE.FOOT_RIGHT] = MotionSensorPos(ply, SENSORBONE.FOOT_RIGHT),
						[SENSORBONE.HAND_RIGHT] = MotionSensorPos(ply, SENSORBONE.HAND_RIGHT),
						[SENSORBONE.SHOULDER] = MotionSensorPos(ply, SENSORBONE.SHOULDER),
						[SENSORBONE.HIP_LEFT] = MotionSensorPos(ply, SENSORBONE.HIP_LEFT),
						[SENSORBONE.HIP_RIGHT] = MotionSensorPos(ply, SENSORBONE.HIP_RIGHT),
						[SENSORBONE.HAND_LEFT] = MotionSensorPos(ply, SENSORBONE.HAND_LEFT),
						[SENSORBONE.ANKLE_RIGHT] = MotionSensorPos(ply, SENSORBONE.ANKLE_RIGHT),
						[SENSORBONE.SPINE] = MotionSensorPos(ply, SENSORBONE.SPINE),
						[SENSORBONE.ELBOW_LEFT] = MotionSensorPos(ply, SENSORBONE.ELBOW_LEFT),
						[SENSORBONE.KNEE_LEFT] = MotionSensorPos(ply, SENSORBONE.KNEE_LEFT),
						[SENSORBONE.HEAD] = MotionSensorPos(ply, SENSORBONE.HEAD),
					}
				end

				local playerExtendedBones = playerExtendedBones[ply:UserID()] or {}
				skeleton[SENSORBONE.SPINE_BASE] = playerExtendedBones[SENSORBONE.SPINE_BASE] or Vector()
				skeleton[SENSORBONE.NECK] = playerExtendedBones[SENSORBONE.NECK] or Vector()
				skeleton[SENSORBONE.SPINE_SHOULDER] = playerExtendedBones[SENSORBONE.SPINE_SHOULDER] or Vector()
				skeleton[SENSORBONE.HAND_TIP_LEFT] = playerExtendedBones[SENSORBONE.HAND_TIP_LEFT] or Vector()
				skeleton[SENSORBONE.THUMB_LEFT] = playerExtendedBones[SENSORBONE.THUMB_LEFT] or Vector()
				skeleton[SENSORBONE.HAND_TIP_RIGHT] = playerExtendedBones[SENSORBONE.HAND_TIP_RIGHT] or Vector()
				skeleton[SENSORBONE.THUMB_RIGHT] = playerExtendedBones[SENSORBONE.THUMB_RIGHT] or Vector()

				return skeleton
			end
		end
	else
		-- Pollyfill motionsensor.GetSkeleton
		if CLIENT then
			chat.AddText(Color(255, 0, 0), "gmcl_rekinect: Server does not support Xbox One Kinect extended bones.")

			if not motionsensor.GetSkeleton then
				function motionsensor.GetSkeleton()
					if not motionsensor.IsActive() then return nil end
					local ply = LocalPlayer()
					local MotionSensorPos = ply.MotionSensorPos

					return {
						[SENSORBONE.SHOULDER_RIGHT] = MotionSensorPos(ply, SENSORBONE.SHOULDER_RIGHT),
						[SENSORBONE.SHOULDER_LEFT] = MotionSensorPos(ply, SENSORBONE.SHOULDER_LEFT),
						[SENSORBONE.HIP] = MotionSensorPos(ply, SENSORBONE.HIP),
						[SENSORBONE.ELBOW_RIGHT] = MotionSensorPos(ply, SENSORBONE.ELBOW_RIGHT),
						[SENSORBONE.KNEE_RIGHT] = MotionSensorPos(ply, SENSORBONE.KNEE_RIGHT),
						[SENSORBONE.WRIST_RIGHT] = MotionSensorPos(ply, SENSORBONE.WRIST_RIGHT),
						[SENSORBONE.ANKLE_LEFT] = MotionSensorPos(ply, SENSORBONE.ANKLE_LEFT),
						[SENSORBONE.FOOT_LEFT] = MotionSensorPos(ply, SENSORBONE.FOOT_LEFT),
						[SENSORBONE.WRIST_LEFT] = MotionSensorPos(ply, SENSORBONE.WRIST_LEFT),
						[SENSORBONE.FOOT_RIGHT] = MotionSensorPos(ply, SENSORBONE.FOOT_RIGHT),
						[SENSORBONE.HAND_RIGHT] = MotionSensorPos(ply, SENSORBONE.HAND_RIGHT),
						[SENSORBONE.SHOULDER] = MotionSensorPos(ply, SENSORBONE.SHOULDER),
						[SENSORBONE.HIP_LEFT] = MotionSensorPos(ply, SENSORBONE.HIP_LEFT),
						[SENSORBONE.HIP_RIGHT] = MotionSensorPos(ply, SENSORBONE.HIP_RIGHT),
						[SENSORBONE.HAND_LEFT] = MotionSensorPos(ply, SENSORBONE.HAND_LEFT),
						[SENSORBONE.ANKLE_RIGHT] = MotionSensorPos(ply, SENSORBONE.ANKLE_RIGHT),
						[SENSORBONE.SPINE] = MotionSensorPos(ply, SENSORBONE.SPINE),
						[SENSORBONE.ELBOW_LEFT] = MotionSensorPos(ply, SENSORBONE.ELBOW_LEFT),
						[SENSORBONE.KNEE_LEFT] = MotionSensorPos(ply, SENSORBONE.KNEE_LEFT),
						[SENSORBONE.HEAD] = MotionSensorPos(ply, SENSORBONE.HEAD),
					}
				end
			end
		end
	end

	gmcl_rekinect_extended_bones_supported_callback(serverSupportsExtendedBones)
end

local serverSupportsExtendedBones = SERVER or util.NetworkStringToID("gmcl_rekinect_extended_bones") ~= 0

if CLIENT and not game.IsDedicated() and not serverSupportsExtendedBones then
	hook.Add("Tick", "gmcl_rekinect_extended_bones", function()
		if util.NetworkStringToID("gmcl_rekinect_extended_bones") ~= 0 then
			init(true)
			hook.Remove("Tick", "gmcl_rekinect_extended_bones")
		end
	end)
else
	init(serverSupportsExtendedBones)
end