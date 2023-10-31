#include "mgba/include/mgba-util/vfs.h"
#include "mgba/include/mgba/core/blip_buf.h"
#include "mgba/include/mgba/core/core.h"
#include "mgba/include/mgba/core/log.h"
#include "mgba/include/mgba/core/timing.h"
#include "mgba/include/mgba/gba/core.h"

#include "mgba/include/mgba/internal/arm/arm.h"
#include "mgba/include/mgba/internal/gba/gba.h"

uint32_t GBALoad32(struct ARMCore *cpu, uint32_t address, int *cycleCounter);