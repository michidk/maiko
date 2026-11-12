loc_20DCD,20,CALL host_clear_scroll_lock
loc_20DE1,20,CALL host_clear_num_lock
loc_D72E5,79,CALL host_keyboard_install
loc_D734F,94,CALL host_keyboard_locks
loc_D7524,16,RET
loc_D7589,16,RET
loc_B9D2F,9,CALL host_prepare_graphics
loc_EB55D,2,test eax,eax|jz loc_EB5C3|mov dl,[eax]
loc_55441,2,RG_VARIANT_JUMP loc_5544F|jz loc_5544F
loc_AD96B,5,RG_VARIANT_JUMP loc_AD9E0|mov ecx,2
loc_D5963,3,test edx,edx|jz loc_D5726|cmp ecx,[edx+4]
loc_BDCAD,4,RG_FONT10_METRIC
loc_ABC0A,8,CALL host_poll_input|cmp word [loc_2055C2],0
loc_AC600,8,CALL host_poll_input|cmp word [loc_2055C2],0
loc_AC95E,8,CALL host_poll_input|cmp word [loc_2055C2],0
loc_117C5B,3,test eax,eax
loc_9AC64,6,mov eax,[eax+0x247]|test eax,eax|jz loc_9AC81
loc_3924E,6,mov eax,[eax+0x247]|test eax,eax|jz loc_394D0
loc_4667E,7,test eax,eax|jz loc_46687|cmp dword [eax+0x272],0
loc_2FAAC,3,mov eax,[eax+0x25]|test eax,eax|jz loc_2FAD7
loc_2FAF8,3,mov eax,[eax+0x25]|test eax,eax|jz loc_2FB23
loc_11A380,168,CALL host_movie_clock_init|RET
loc_C84C0,305,CALL loc_C85F4|CALL host_audio_init|RET
loc_CB580,3,CALL host_audio_startup|RET
loc_D03A0,2,CALL host_audio_cd_open|RET
loc_B22D2,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
loc_B25CA,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
loc_B25D4,8,CALL host_poll_input|cmp word [loc_2055C2],0
loc_B25A6,8,CALL host_poll_input|cmp word [loc_205612],0
loc_B263A,8,CALL host_poll_input|cmp word [loc_2055F8],0
loc_C7C88,8,CALL host_poll_input|cmp word [loc_2055C2],0
loc_C7C9B,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
loc_2418D,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
loc_2419C,2,CALL host_poll_input|xor eax,eax
loc_B5833,8,CALL host_poll_input|cmp word [loc_2055C2],0
loc_F1EE3,6,CALL host_audio_callbacks_prepare|mov edx,[loc_1AA258]
loc_BCB2D,2,mov eax,[eax]|test eax,eax|jz loc_BCB4F
