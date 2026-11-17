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

package me.zodac.folding.api.tc.validation;

import static java.util.stream.Collectors.toList;
import static me.zodac.folding.api.util.StringUtils.isBlank;

import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.stream.Stream;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.tc.HardwareMake;
import me.zodac.folding.api.tc.HardwareType;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.rest.api.tc.request.HardwareRequest;


public final class HardwareValidator {

    // The hardware with the highest PPD will have a multiplier of <b>1.00</b>, and all others will be based on that
    private static final double MINIMUM_MULTIPLIER_VALUE = 1.00D;
    private static final long MINIMUM_AVERAGE_PPD_VALUE = 1L;

    private HardwareValidator() {

    }

    
    public static ValidationResult<Hardware> validateCreate(final HardwareRequest hardwareRequest, final Collection<Hardware> allHardware) {
        if (hardwareRequest == null) {
            return ValidationResult.nullObject();
        }

        // The hardwareName must be unique
        final Optional<Hardware> hardwareWithMatchingName = getHardwareWithName(hardwareRequest.getHardwareName(), allHardware);
        if (hardwareWithMatchingName.isPresent()) {
            return ValidationResult.conflictingWith(hardwareRequest, hardwareWithMatchingName.get(), List.of("hardwareName"));
        }

        final List<String> failureMessages = Stream.of(
                hardwareName(hardwareRequest),
                displayName(hardwareRequest),
                hardwareMake(hardwareRequest),
                hardwareType(hardwareRequest),
                multiplier(hardwareRequest),
                averagePpd(hardwareRequest)
            )
            .filter(Objects::nonNull)
            .collect(toList());

        if (!failureMessages.isEmpty()) {
            return ValidationResult.failure(hardwareRequest, failureMessages);
        }

        final Hardware convertedHardware = Hardware.createWithoutId(hardwareRequest);
        return ValidationResult.successful(convertedHardware);
    }

    
    public static ValidationResult<Hardware> validateUpdate(final HardwareRequest hardwareRequest,
                                                            final Hardware existingHardware,
                                                            final Collection<Hardware> allHardware) {
        if (hardwareRequest == null || existingHardware == null) {
            return ValidationResult.nullObject();
        }

        // The hardwareName must be unique, unless replacing the same hardware
        final Optional<Hardware> hardwareWithMatchingName = getHardwareWithName(hardwareRequest.getHardwareName(), allHardware);
        if (hardwareWithMatchingName.isPresent() && hardwareWithMatchingName.get().getId() != existingHardware.getId()) {
            return ValidationResult.conflictingWith(hardwareRequest, hardwareWithMatchingName.get(), List.of("hardwareName"));
        }

        final List<String> failureMessages = Stream.of(
                hardwareName(hardwareRequest),
                displayName(hardwareRequest),
                hardwareMake(hardwareRequest),
                hardwareType(hardwareRequest),
                multiplier(hardwareRequest),
                averagePpd(hardwareRequest)
            )
            .filter(Objects::nonNull)
            .collect(toList());

        if (!failureMessages.isEmpty()) {
            return ValidationResult.failure(hardwareRequest, failureMessages);
        }

        final Hardware convertedHardware = Hardware.createWithoutId(hardwareRequest);
        return ValidationResult.successful(convertedHardware);
    }

    
    public static ValidationResult<Hardware> validateDelete(final Hardware hardware, final Collection<User> allUsers) {
        final Collection<User> usersWithMatchingHardware = getUsersWithHardware(hardware.getId(), allUsers);

        if (!usersWithMatchingHardware.isEmpty()) {
            return ValidationResult.usedBy(hardware, usersWithMatchingHardware);
        }

        return ValidationResult.successful(hardware);
    }

    private static Optional<Hardware> getHardwareWithName(final String hardwareName, final Collection<Hardware> allHardware) {
        if (isBlank(hardwareName)) {
            return Optional.empty();
        }

        return allHardware
            .stream()
            .filter(hardware -> hardware.getHardwareName().equalsIgnoreCase(hardwareName))
            .findAny();
    }

    private static Collection<User> getUsersWithHardware(final int hardwareId, final Collection<User> allUsers) {
        if (hardwareId == Hardware.EMPTY_HARDWARE_ID) {
            return Collections.emptyList();
        }

        return allUsers
            .stream()
            .filter(user -> user.getHardware().getId() == hardwareId)
            .map(User::hidePasskey)
            .collect(toList());
    }

    private static String hardwareName(final HardwareRequest hardwareRequest) {
        return isBlank(hardwareRequest.getHardwareName())
            ? "Field 'hardwareName' must not be empty"
            : null;
    }

    private static String displayName(final HardwareRequest hardwareRequest) {
        return isBlank(hardwareRequest.getDisplayName())
            ? "Field 'displayName' must not be empty"
            : null;
    }

    private static String hardwareMake(final HardwareRequest hardwareRequest) {
        return HardwareMake.get(hardwareRequest.getHardwareMake()) == HardwareMake.INVALID
            ? String.format("Field 'hardwareMake' must be one of: %s", HardwareMake.getAllValues())
            : null;
    }

    private static String hardwareType(final HardwareRequest hardwareRequest) {
        return HardwareType.get(hardwareRequest.getHardwareType()) == HardwareType.INVALID
            ? String.format("Field 'hardwareType' must be one of: %s", HardwareType.getAllValues())
            : null;
    }

    private static String multiplier(final HardwareRequest hardwareRequest) {
        return hardwareRequest.getMultiplier() >= MINIMUM_MULTIPLIER_VALUE
            ? null
            : String.format("Field 'multiplier' must be %.2f or higher", MINIMUM_MULTIPLIER_VALUE);
    }

    private static String averagePpd(final HardwareRequest hardwareRequest) {
        return hardwareRequest.getAveragePpd() >= MINIMUM_AVERAGE_PPD_VALUE
            ? null
            : String.format("Field 'averagePpd' must be %d or higher", MINIMUM_AVERAGE_PPD_VALUE);
    }
}
