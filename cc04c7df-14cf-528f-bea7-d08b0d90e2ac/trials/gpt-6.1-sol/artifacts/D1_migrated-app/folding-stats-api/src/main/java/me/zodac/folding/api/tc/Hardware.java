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

package me.zodac.folding.api.tc;

import java.util.Objects;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.ToString;
import me.zodac.folding.api.ResponsePojo;
import me.zodac.folding.rest.api.tc.request.HardwareRequest;



@AllArgsConstructor(access = AccessLevel.PRIVATE)
@Builder
@Getter
@EqualsAndHashCode
@ToString(doNotUseGetters = true)
public class Hardware implements ResponsePojo {

    
    public static final int EMPTY_HARDWARE_ID = 0;

    private final int id;
    private final String hardwareName;
    private final String displayName;
    private final HardwareMake hardwareMake;
    private final HardwareType hardwareType;
    private final double multiplier;
    private final long averagePpd;

    
    public static Hardware create(final int hardwareId,
                                  final String hardwareName,
                                  final String displayName,
                                  final HardwareMake hardwareMake,
                                  final HardwareType hardwareType,
                                  final double multiplier,
                                  final long averagePpd) {
        return new Hardware(hardwareId, hardwareName, displayName, hardwareMake, hardwareType, multiplier, averagePpd);
    }

    
    public static Hardware createWithoutId(final String hardwareName,
                                           final String displayName,
                                           final HardwareMake hardwareMake,
                                           final HardwareType hardwareType,
                                           final double multiplier,
                                           final long averagePpd) {
        return create(EMPTY_HARDWARE_ID, hardwareName, displayName, hardwareMake, hardwareType, multiplier, averagePpd);
    }

    
    public static Hardware createWithoutId(final HardwareRequest hardwareRequest) {
        return createWithoutId(
            hardwareRequest.getHardwareName(),
            hardwareRequest.getDisplayName(),
            HardwareMake.get(hardwareRequest.getHardwareMake()),
            HardwareType.get(hardwareRequest.getHardwareType()),
            hardwareRequest.getMultiplier(),
            hardwareRequest.getAveragePpd()
        );
    }

    
    public static Hardware updateWithId(final int hardwareId, final Hardware hardware) {
        return create(
            hardwareId,
            hardware.hardwareName,
            hardware.displayName,
            hardware.hardwareMake,
            hardware.hardwareType,
            hardware.multiplier,
            hardware.averagePpd
        );
    }

    
    public boolean isEqualRequest(final HardwareRequest hardwareRequest) {
        return Double.compare(multiplier, hardwareRequest.getMultiplier()) == 0
            && Objects.equals(hardwareName, hardwareRequest.getHardwareName())
            && Objects.equals(displayName, hardwareRequest.getDisplayName())
            && Objects.equals(hardwareMake.toString(), hardwareRequest.getHardwareMake())
            && Objects.equals(hardwareType.toString(), hardwareRequest.getHardwareType())
            && averagePpd == hardwareRequest.getAveragePpd();
    }
}
