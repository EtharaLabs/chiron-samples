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

import static java.util.stream.Collectors.toUnmodifiableList;

import java.util.Collection;
import java.util.Collections;
import java.util.EnumSet;
import java.util.Set;
import java.util.stream.Stream;
import me.zodac.folding.api.util.EnvironmentVariableUtils;


public enum Category {

    
    AMD_GPU(getCategoryCount("USERS_IN_AMD_GPU"), HardwareMake.AMD, HardwareType.GPU),

    
    NVIDIA_GPU(getCategoryCount("USERS_IN_NVIDIA_GPU"), HardwareMake.NVIDIA, HardwareType.GPU),

    
    WILDCARD(getCategoryCount("USERS_IN_WILDCARD"), HardwareMake.getAllValues(), HardwareType.getAllValues()),

    
    INVALID(0, Collections.emptySet(), Collections.emptySet());

    private static final int DEFAULT_USERS_PER_CATEGORY = 1;

    private static final Collection<Category> ALL_VALUES = Stream.of(values())
        .filter(value -> value != INVALID)
        .collect(toUnmodifiableList());

    private final int permittedUsers;
    private final EnumSet<HardwareMake> supportedHardwareMakes;
    private final EnumSet<HardwareType> supportedHardwareTypes;

    
    Category(final int permittedUsers, final Collection<HardwareMake> hardwareMakes, final Collection<HardwareType> hardwareTypes) {
        this.permittedUsers = permittedUsers;
        supportedHardwareMakes = hardwareMakes.isEmpty() ? EnumSet.noneOf(HardwareMake.class) : EnumSet.copyOf(hardwareMakes);
        supportedHardwareTypes = hardwareTypes.isEmpty() ? EnumSet.noneOf(HardwareType.class) : EnumSet.copyOf(hardwareTypes);
    }

    
    Category(final int permittedUsers, final HardwareMake hardwareMake, final HardwareType hardwareType) {
        this.permittedUsers = permittedUsers;
        supportedHardwareMakes = EnumSet.of(hardwareMake);
        supportedHardwareTypes = EnumSet.of(hardwareType);
    }

    
    public static Collection<Category> getAllValues() {
        return ALL_VALUES;
    }

    
    public static int maximumPermittedAmountForAllCategories() {
        return ALL_VALUES.stream()
            .mapToInt(value -> value.permittedUsers)
            .sum();
    }

    
    public static Category get(final String input) {
        return ALL_VALUES
            .stream()
            .filter(category -> category.toString().equalsIgnoreCase(input))
            .findAny()
            .orElse(Category.INVALID);
    }

    
    public int permittedUsers() {
        return permittedUsers;
    }

    
    public boolean isHardwareMakeSupported(final HardwareMake hardwareMake) {
        return supportedHardwareMakes.contains(hardwareMake);
    }

    
    public boolean isHardwareTypeSupported(final HardwareType hardwareType) {
        return supportedHardwareTypes.contains(hardwareType);
    }

    
    public Set<HardwareMake> supportedHardwareMakes() {
        return supportedHardwareMakes;
    }

    
    public Set<HardwareType> supportedHardwareTypes() {
        return supportedHardwareTypes;
    }

    private static int getCategoryCount(final String categoryVariable) {
        return convertToIntOrDefault(EnvironmentVariableUtils.get(categoryVariable));
    }

    private static int convertToIntOrDefault(final String input) {
        try {
            return Integer.parseInt(input);
        } catch (final NumberFormatException e) {
            return DEFAULT_USERS_PER_CATEGORY;
        }
    }
}