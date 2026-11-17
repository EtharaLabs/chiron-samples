/*
 * MIT License
 *
 * Copyright (c) 2021-2022 zodac.me
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

package me.zodac.folding.service.tc.lars;

import org.springframework.stereotype.Service;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Lazy;

import static java.util.stream.Collectors.toSet;
import static me.zodac.folding.api.util.NumberUtils.formatWithCommas;

import java.util.Collection;
import java.util.List;
import java.util.Map;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.tc.HardwareMake;
import me.zodac.folding.api.tc.HardwareType;
import me.zodac.folding.api.tc.lars.LarsGpu;
import me.zodac.folding.service.api.FoldingStatsCore;
import me.zodac.folding.lars.HardwareSplitter;
import me.zodac.folding.lars.LarsGpuRetriever;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@Service
public class LarsHardwareUpdater {

    private static final Logger LOGGER = LogManager.getLogger();
    @org.springframework.beans.factory.annotation.Value("${LARS_URL_ROOT:https://folding.lar.systems}")
    private String larsUrlRoot;

    @Autowired
    @Lazy
    private FoldingStatsCore foldingStatsCore;

    public void retrieveHardwareAndPersist() {
        final String gpuDbUrl = larsUrlRoot + "/gpu_ppd/overall_ranks";
        final List<LarsGpu> larsGpus = LarsGpuRetriever.retrieveGpus(gpuDbUrl);
        LOGGER.debug("Retrieved GPUs from LARS DB: {}", larsGpus);
        LOGGER.info("Retrieved {} GPUs from LARS DB", larsGpus.size());

        if (larsGpus.isEmpty()) {
            LOGGER.warn("No GPUs retrieved from LARs DB");
            return;
        }

        final LarsGpu bestGpu = larsGpus.get(0);
        final long bestPpd = bestGpu.getAveragePpd();
        LOGGER.info("Best PPD is '{}' for '{}', will compare to this", formatWithCommas(bestPpd), bestGpu.getModelInfo());

        final Collection<Hardware> lars = larsGpus
            .stream()
            .map(larsGpu -> toHardware(larsGpu, bestPpd))
            .collect(toSet());
        final Collection<Hardware> existing = foldingStatsCore.getAllHardware();

        for (final Hardware hardware : HardwareSplitter.toDelete(lars, existing)) {
            foldingStatsCore.deleteHardware(hardware);
            LOGGER.info("Deleted hardware '{}' (ID: {})", hardware.getHardwareName(), hardware.getId());
        }

        for (final Map.Entry<Hardware, Hardware> entry : HardwareSplitter.toUpdate(lars, existing).entrySet()) {
            try {
                final Hardware updatedHardware = entry.getKey();
                final Hardware existingHardware = entry.getValue();
                final Hardware updatedHardwareWithId = Hardware.updateWithId(existingHardware.getId(), updatedHardware);

                foldingStatsCore.updateHardware(updatedHardwareWithId, existingHardware);
                LOGGER.info("LARS updated hardware\n" +
                        "ID: {}\n" +
                        "{}\n" +
                        "Multiplier: {} -> {}\n" +
                        "Average PPD: {} -> {}\n",
                    existingHardware.getId(),
                    updatedHardware.getHardwareName(),
                    existingHardware.getMultiplier(),
                    updatedHardware.getMultiplier(),
                    formatWithCommas(existingHardware.getAveragePpd()),
                    formatWithCommas(updatedHardware.getAveragePpd()));
            } catch (final Exception e) {
                LOGGER.warn("Unexpected error connecting to Folding@Home stats to verify new hardware", e);
            }
        }

        for (final Hardware hardware : HardwareSplitter.toCreate(lars, existing)) {
            final Hardware createdHardware = foldingStatsCore.createHardware(hardware);
            LOGGER.info("Created hardware '{}' (ID: {})", createdHardware.getHardwareName(), createdHardware.getId());
        }
    }

    private static Hardware toHardware(final LarsGpu larsGpu, final long bestPpd) {
        return Hardware.builder()
            .hardwareName(larsGpu.getModelInfo())
            .displayName(larsGpu.getDisplayName())
            .hardwareMake(HardwareMake.get(larsGpu.getManufacturer()))
            .hardwareType(HardwareType.GPU)
            .multiplier(getMultiplier(bestPpd, larsGpu.getAveragePpd()))
            .averagePpd(larsGpu.getAveragePpd())
            .build();
    }

    private static double getMultiplier(final long bestPpd, final long currentPpd) {
        return roundDoubleToTwoPlaces((double) bestPpd / currentPpd);
    }

    private static double roundDoubleToTwoPlaces(final double input) {
        final String doubleAsString = String.format("%.2f", input);
        return Double.parseDouble(doubleAsString);
    }
}
