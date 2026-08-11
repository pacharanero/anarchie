/*
 * SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
 * SPDX-License-Identifier: AGPL-3.0-or-later
 */

package org.anarchie.oracle;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.nedap.archie.adlparser.ADLParser;
import com.nedap.archie.aom.Archetype;
import com.nedap.archie.aom.OperationalTemplate;
import com.nedap.archie.flattener.Flattener;
import com.nedap.archie.flattener.FlattenerConfiguration;
import com.nedap.archie.flattener.InMemoryFullArchetypeRepository;
import com.nedap.archie.json.ArchieJacksonConfiguration;
import com.nedap.archie.json.JacksonUtil;
import com.nedap.archie.rm.composition.Composition;
import com.nedap.archie.rminfo.ArchieRMInfoLookup;
import com.nedap.archie.rmobjectvalidator.RMObjectValidationMessage;
import com.nedap.archie.rmobjectvalidator.RMObjectValidator;
import com.nedap.archie.rmobjectvalidator.ValidationConfiguration;
import com.nedap.archie.xml.JAXBUtil;
import org.openehr.referencemodels.BuiltinReferenceModels;

import javax.xml.bind.JAXBElement;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class ArchieValidator {
    private ArchieValidator() {
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 2) {
            System.err.println("usage: ArchieValidator <template.adls|template.optx> <composition.json>");
            System.exit(2);
        }

        Path templatePath = Paths.get(args[0]);
        Path compositionPath = Paths.get(args[1]);
        OperationalTemplate template = loadTemplate(templatePath);

        ObjectMapper mapper = JacksonUtil.getObjectMapper(
                ArchieJacksonConfiguration.createStandardsCompliant());
        Composition composition = mapper.readValue(compositionPath.toFile(), Composition.class);
        RMObjectValidator validator = new RMObjectValidator(
                ArchieRMInfoLookup.getInstance(),
                new InMemoryFullArchetypeRepository(),
                new ValidationConfiguration.Builder().build());
        List<RMObjectValidationMessage> messages = validator.validate(template, composition);

        List<Map<String, Object>> serializedMessages = new ArrayList<>();
        for (RMObjectValidationMessage message : messages) {
            Map<String, Object> serialized = new LinkedHashMap<>();
            serialized.put("type", message.getType().name());
            serialized.put("path", message.getPath());
            serialized.put("archetype_path", message.getArchetypePath());
            serialized.put("archetype_id", message.getArchetypeId());
            serialized.put("message", message.getMessage());
            serializedMessages.add(serialized);
        }

        Map<String, Object> result = new LinkedHashMap<>();
        result.put("valid", messages.isEmpty());
        result.put("messages", serializedMessages);
        System.out.println(mapper.writeValueAsString(result));
    }

    private static OperationalTemplate loadTemplate(Path templatePath) throws Exception {
        if (templatePath.getFileName().toString().endsWith(".adls")) {
            Archetype archetype;
            try (java.io.InputStream input = Files.newInputStream(templatePath)) {
                archetype = new ADLParser(BuiltinReferenceModels.getMetaModelProvider()).parse(input);
            }
            InMemoryFullArchetypeRepository repository = new InMemoryFullArchetypeRepository();
            repository.addArchetype(archetype);
            return (OperationalTemplate) new Flattener(
                    repository,
                    BuiltinReferenceModels.getMetaModelProvider(),
                    FlattenerConfiguration.forOperationalTemplate()).flatten(archetype);
        }

        Object templateValue;
        try (java.io.InputStream input = Files.newInputStream(templatePath)) {
            templateValue = JAXBUtil.getArchieJAXBContext().createUnmarshaller().unmarshal(input);
        }
        if (templateValue instanceof JAXBElement) {
            templateValue = ((JAXBElement<?>) templateValue).getValue();
        }
        if (!(templateValue instanceof OperationalTemplate)) {
            throw new IllegalArgumentException("OPT2 XML did not contain an OperationalTemplate");
        }
        return (OperationalTemplate) templateValue;
    }
}
